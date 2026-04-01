# Audio Recorder

Records microphone + system audio and transcribes with Gemini Flash. Sessions (WAV + transcript) can be auto-pushed to a GitHub repo.

## How It Works

1. **Audio capture** — opens your microphone and (optionally) a BlackHole loopback device to capture system audio. Both streams are mixed into a single mono 16 kHz int16 buffer.
2. **Gemini transcription** — after you stop recording, the saved WAV is sent to Gemini 2.5 Flash for verbatim transcription with speaker labels and timestamps. Long recordings are automatically split into chunks.
3. **GitHub push** — session files (`.wav`, `_gemini.txt`) are committed and pushed to a configurable GitHub repo.

## Prerequisites

- **Python 3.9+**
- **PortAudio** — required by PyAudio
  ```bash
  brew install portaudio
  ```
- **BlackHole** (optional, for system audio capture)
  ```bash
  brew install --cask blackhole-2ch
  ```
  After installing, reboot, then create a **Multi-Output Device** in Audio MIDI Setup that combines BlackHole with your speakers. Set macOS output to that device while recording.

## Setup

```bash
# clone the repo
git clone <repo-url> && cd reefrecord

# create a virtual environment
python3 -m venv venv
source venv/bin/activate

# install dependencies
pip install pyaudio requests python-dotenv
```

Create a `.env` file in the project root:

```env
GEMINI_API_KEY=your_gemini_api_key             # required for transcription
GH_SESSIONS_REPO=git@github.com:user/repo.git # optional — skip push if empty
GH_SESSIONS_FOLDER=sessions                   # subfolder inside the repo (default: sessions)
SYSTEM_AUDIO_DEVICE=                           # device name/index override (default: auto-detect BlackHole)
MIC_DEVICE=                                    # mic name/index override (default: system default mic)
```

## Usage

```bash
python recorder.py
```

You'll see a simple menu:

```
=== audio recorder ===

[s]tart / [q]uit:
```

Press **s** to start a session. The recorder prints detected devices and begins recording:

```
  system audio: [4] BlackHole 2ch
  microphone:   [1] MacBook Pro Microphone

  recording → sessions/session_20260319_140000.wav
  press enter to stop
```

Press **Enter** to stop. The recorder saves the audio, transcribes it with Gemini, and optionally pushes to GitHub:

| File | Contents |
|------|----------|
| `session_<timestamp>.wav` | Raw audio recording (16 kHz mono) |
| `session_<timestamp>_gemini.txt` | Gemini Flash transcript with speaker labels and timestamps |

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `GEMINI_API_KEY` | Yes | Google Gemini API key for transcription |
| `GH_SESSIONS_REPO` | No | Git remote URL to push session files to |
| `GH_SESSIONS_FOLDER` | No | Target folder in the repo (default: `sessions`) |
| `SYSTEM_AUDIO_DEVICE` | No | Override system audio device by name or index |
| `MIC_DEVICE` | No | Override microphone device by name or index |
