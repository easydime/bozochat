import { WebSocketServer } from 'ws';
import express from 'express';
import { Client, GatewayIntentBits, SlashCommandBuilder, REST, Routes, AttachmentBuilder } from 'discord.js';
import dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import https from 'https';
import http from 'http';

dotenv.config();

const PORT = process.env.PORT || 3001;

// Store connected clients
const clients = new Map();

// Create HTTP server
const app = express();
const server = app.listen(PORT, () => {
  console.log(`✓ BozoChat server started on port ${PORT}`);
});

// Create WebSocket server
const wss = new WebSocketServer({ server });

console.log('✓ WebSocket server initialized');

// Handle WebSocket connections
wss.on('connection', (ws, req) => {
  const clientId = Date.now().toString();
  const clientIp = req.socket.remoteAddress;

  console.log(`[WS] New connection: ${clientId} from ${clientIp}`);

  clients.set(clientId, {
    ws,
    userId: null,
    connectedAt: new Date(),
    ip: clientIp
  });

  ws.send(JSON.stringify({
    type: 'connected',
    message: 'Connected to BozoChat server',
    clientId
  }));

  ws.on('message', (data) => {
    try {
      const message = JSON.parse(data.toString());
      handleClientMessage(clientId, message);
    } catch (error) {
      console.error(`[WS] Error parsing message from ${clientId}:`, error.message);
    }
  });

  ws.on('close', () => {
    const client = clients.get(clientId);
    console.log(`[WS] Disconnected: ${clientId} (${client?.userId || 'anonymous'})`);
    clients.delete(clientId);
  });

  ws.on('error', (error) => {
    console.error(`[WS] Error on ${clientId}:`, error.message);
  });
});

// Handle client messages
function handleClientMessage(clientId, message) {
  const client = clients.get(clientId);

  // Check if client exists (could have disconnected)
  if (!client) {
    console.error(`[WS] Client ${clientId} not found, ignoring message`);
    return;
  }

  switch (message.type) {
    case 'auth':
      client.userId = message.userId;
      console.log(`[WS] Client ${clientId} authenticated as: ${message.userId}`);
      break;

    case 'pong':
      // Response to ping
      break;

    default:
      console.log(`[WS] Unhandled message from ${clientId}:`, message.type);
  }
}

// Broadcast to all clients
export function broadcastToAll(data) {
  let sent = 0;
  clients.forEach((client, clientId) => {
    if (client.ws.readyState === 1) { // WebSocket.OPEN = 1
      client.ws.send(JSON.stringify(data));
      sent++;
    }
  });
  return sent;
}

// Send to specific user
export function sendToUser(userId, data) {
  for (const [clientId, client] of clients.entries()) {
    if (client.userId === userId && client.ws.readyState === 1) {
      client.ws.send(JSON.stringify(data));
      return true;
    }
  }
  return false;
}

// Get connected clients
export function getConnectedClients() {
  const clientList = [];
  clients.forEach((client, clientId) => {
    clientList.push({
      id: clientId,
      userId: client.userId || 'anonymous',
      connectedAt: client.connectedAt,
      ip: client.ip
    });
  });
  return clientList;
}

// Periodic ping
setInterval(() => {
  clients.forEach((client, clientId) => {
    if (client.ws.readyState === 1) {
      client.ws.send(JSON.stringify({ type: 'ping' }));
    }
  });
}, 30000);

// HTTP routes
app.get('/', (req, res) => {
  res.json({
    status: 'online',
    name: 'BozoChat Server',
    version: '1.0.0',
    connectedClients: clients.size
  });
});

app.get('/clients', (req, res) => {
  res.json({
    count: clients.size,
    clients: getConnectedClients()
  });
});

// Discord Bot Setup
const discordClient = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
  ]
});

// Slash commands
const commands = [
  new SlashCommandBuilder()
    .setName('send')
    .setDescription('Send an image/video and/or text to everyone\'s screen')
    .addAttachmentOption(option =>
      option.setName('media')
        .setDescription('The image or video to send (optional)')
        .setRequired(false))
    .addStringOption(option =>
      option.setName('message')
        .setDescription('Message to display (optional)')
        .setRequired(false))
    .addIntegerOption(option =>
      option.setName('duration')
        .setDescription('Display duration in seconds (default: 5)')
        .setRequired(false)
        .setMinValue(1)
        .setMaxValue(30)),

  new SlashCommandBuilder()
    .setName('status')
    .setDescription('Check how many clients are connected'),
];

// Register commands when bot is ready
discordClient.once('ready', async () => {
  console.log(`✓ Discord bot connected: ${discordClient.user.tag}`);

  const rest = new REST({ version: '10' }).setToken(process.env.DISCORD_TOKEN);

  try {
    console.log('📝 Registering slash commands...');

    const data = await rest.put(
      Routes.applicationCommands(discordClient.user.id),
      { body: commands }
    );

    console.log(`✓ ${data.length} commands registered successfully`);
    console.log(`🤖 Bot ready! Invite URL:`);
    console.log(`https://discord.com/api/oauth2/authorize?client_id=${discordClient.user.id}&permissions=2048&scope=bot%20applications.commands`);
  } catch (error) {
    console.error('❌ Error registering commands:', error);
  }
});

// Handle slash commands
discordClient.on('interactionCreate', async (interaction) => {
  if (!interaction.isChatInputCommand()) return;

  const { commandName } = interaction;

  try {
    if (commandName === 'send') {
      await handleSendCommand(interaction);
    } else if (commandName === 'status') {
      await handleStatusCommand(interaction);
    }
  } catch (error) {
    console.error(`Error executing /${commandName}:`, error);

    if (!interaction.replied && !interaction.deferred) {
      await interaction.reply({
        content: '❌ An error occurred',
        ephemeral: true
      });
    }
  }
});

// Handle /send command
async function handleSendCommand(interaction) {
  await interaction.deferReply();

  const attachment = interaction.options.getAttachment('media');
  const message = interaction.options.getString('message');
  const duration = interaction.options.getInteger('duration') || 5;

  // Check that at least one is provided
  if (!attachment && !message) {
    await interaction.editReply('❌ Please provide at least a message or media!');
    return;
  }

  // Prepare payload data
  const payloadData = {
    sender: interaction.user.username,
    message: message || '',
    duration: duration * 1000
  };

  // Download media if provided
  try {
    if (attachment) {
      // Validate attachment type with strict whitelist
      const ALLOWED_MIME_TYPES = [
        'image/jpeg', 'image/png', 'image/gif', 'image/webp',
        'video/mp4', 'video/webm', 'video/quicktime'
      ];

      if (!attachment.contentType || !ALLOWED_MIME_TYPES.includes(attachment.contentType)) {
        await interaction.editReply('❌ File type not supported! Allowed: JPEG, PNG, GIF, WebP, MP4, WebM');
        return;
      }

      // Check file size before downloading
      if (attachment.size > MAX_FILE_SIZE) {
        await interaction.editReply(`❌ File too large! Maximum size: ${MAX_FILE_SIZE / 1024 / 1024}MB`);
        return;
      }

      const mediaBuffer = await downloadMedia(attachment.url);
      const mediaData = mediaBuffer.toString('base64');

      payloadData.mediaType = attachment.contentType;
      payloadData.mediaFilename = attachment.filename;
      payloadData.mediaData = mediaData;
    }

    // Prepare payload
    const payload = {
      type: 'notification',
      data: payloadData
    };

    // Broadcast to all clients
    const sent = broadcastToAll(payload);

    if (sent === 0) {
      await interaction.editReply('❌ No clients are connected!');
      return;
    }

    // Build response fields
    const fields = [];
    if (message && message.trim()) {
      fields.push({ name: 'Message', value: message, inline: false });
    }
    if (attachment && attachment.filename) {
      fields.push({ name: 'Media', value: attachment.filename, inline: true });
    }
    fields.push({ name: 'Recipients', value: `${sent} client(s)`, inline: true });
    fields.push({ name: 'Duration', value: `${duration}s`, inline: true });

    await interaction.editReply({
      embeds: [{
        color: 0x00ff00,
        title: '✅ Sent!',
        fields: fields,
        timestamp: new Date()
      }]
    });

  } catch (error) {
    console.error('Error sending media:', error);
    await interaction.editReply('❌ Failed to download or send media');
  }
}

// Handle /status command
async function handleStatusCommand(interaction) {
  const clientsList = getConnectedClients();

  await interaction.reply({
    embeds: [{
      color: 0x5865f2,
      title: '📊 BozoChat Status',
      fields: [
        { name: 'Connected Clients', value: `${clientsList.length}`, inline: true },
        { name: 'Server Uptime', value: formatUptime(process.uptime()), inline: true }
      ],
      timestamp: new Date()
    }]
  });
}

// Download media from URL with timeout
const DOWNLOAD_TIMEOUT = 30000; // 30 seconds
const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50 MB

function downloadMedia(url) {
  return new Promise((resolve, reject) => {
    const protocol = url.startsWith('https') ? https : http;

    const request = protocol.get(url, (res) => {
      const chunks = [];
      let totalSize = 0;

      res.on('data', (chunk) => {
        totalSize += chunk.length;
        if (totalSize > MAX_FILE_SIZE) {
          request.destroy();
          reject(new Error(`File too large (max ${MAX_FILE_SIZE / 1024 / 1024}MB)`));
          return;
        }
        chunks.push(chunk);
      });
      res.on('end', () => resolve(Buffer.concat(chunks)));
      res.on('error', reject);
    });

    request.on('error', reject);

    // Timeout handling
    request.setTimeout(DOWNLOAD_TIMEOUT, () => {
      request.destroy();
      reject(new Error('Download timeout'));
    });
  });
}

// Format uptime
function formatUptime(seconds) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  const parts = [];
  if (days > 0) parts.push(`${days}d`);
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0 || parts.length === 0) parts.push(`${secs}s`);

  return parts.join(' ');
}

// Start Discord bot
const discordToken = process.env.DISCORD_TOKEN;

if (!discordToken) {
  console.error('❌ DISCORD_TOKEN missing in .env file');
  console.log('Create a .env file with your Discord token:');
  console.log('DISCORD_TOKEN=your_token_here');
  process.exit(1);
}

discordClient.login(discordToken).catch((error) => {
  console.error('❌ Error starting Discord bot:', error);
  process.exit(1);
});

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\n⏹️  Shutting down...');

  broadcastToAll({
    type: 'server-shutdown',
    message: 'Server is shutting down'
  });

  clients.forEach((client) => {
    client.ws.close();
  });

  server.close(() => {
    console.log('✓ Server stopped');
    process.exit(0);
  });
});

console.log('✓ BozoChat server is running!');
console.log(`📡 WebSocket: ws://localhost:${PORT}`);
console.log(`🌐 HTTP: http://localhost:${PORT}`);
