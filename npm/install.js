const fs = require('fs');
const axios = require('axios');
const tar = require('tar');

const packageJson = require('./package.json');
const version = packageJson.version;
const REPO = 'jakvbs/codex-mcp-rs';

const platformMap = {
  'darwin': 'Darwin',
  'linux': 'Linux'
};

const archMap = {
  'x64': 'x86_64',
  'arm64': 'arm64'
};

const platform = platformMap[process.platform];
const arch = archMap[process.arch];

if (!platform || !arch) {
  console.error(`Unsupported platform: ${process.platform} ${process.arch}`);
  process.exit(1);
}

const binaryName = 'codex-mcp-rs';
const fileName = `codex-mcp-rs_${platform}_${arch}.tar.gz`;
const downloadUrl = `https://github.com/${REPO}/releases/download/v${version}/${fileName}`;

console.log(`Downloading ${downloadUrl}...`);

async function download() {
  const writer = fs.createWriteStream(fileName);

  try {
    const response = await axios({
      url: downloadUrl,
      method: 'GET',
      responseType: 'stream'
    });

    response.data.pipe(writer);

    return new Promise((resolve, reject) => {
      writer.on('finish', resolve);
      writer.on('error', reject);
    });
  } catch (error) {
    console.error(`Error downloading binary: ${error.message}`);
    process.exit(1);
  }
}

async function extract() {
  console.log(`Extracting ${fileName}...`);

  await tar.x({
    file: fileName,
    cwd: '.'
  });
  cleanup();
}

function cleanup() {
  fs.unlinkSync(fileName);
  fs.chmodSync(binaryName, 0o755);
  console.log('Installation complete!');
}

download().then(extract);
