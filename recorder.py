import os
import json
import struct
import threading
import wave
import base64
import pyaudio
import requests
from datetime import datetime
from dotenv import load_dotenv

load_dotenv()

import subprocess
import tempfile
import shutil

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY", "")
GH_SESSIONS_REPO = os.getenv("GH_SESSIONS_REPO", "").strip()
GH_SESSIONS_FOLDER = os.getenv("GH_SESSIONS_FOLDER", "sessions").strip()

RATE = 16000
CHANNELS = 1
CHUNK = 4096
FORMAT = pyaudio.paInt16

SESSION_DIR = "sessions"
os.makedirs(SESSION_DIR, exist_ok=True)

HAL_PLUGIN_DIR = "/Library/Audio/Plug-Ins/HAL"
BLACKHOLE_DRIVER_BUNDLES = ("BlackHole2ch.driver", "BlackHole16ch.driver", "BlackHole64ch.driver")

SYSTEM_AUDIO_DEVICE = os.getenv("SYSTEM_AUDIO_DEVICE", "").strip()
MIC_DEVICE = os.getenv("MIC_DEVICE", "").strip()
BLACKHOLE_FRAGMENTS = ("blackhole", "black hole")


def get_devices(pa):
    """Snapshot PortAudio devices with stable indexes."""
    devices = []
    for i in range(pa.get_device_count()):
        info = dict(pa.get_device_info_by_index(i))
        info["index"] = i
        devices.append(info)
    return devices


def resolve_device(devices, selector, require_input=True):
    """Resolve a device by index or case-insensitive name fragment."""
    if not selector:
        return None

    candidates = devices
    if require_input:
        candidates = [info for info in devices if info["maxInputChannels"] > 0]

    if selector.isdigit():
        idx = int(selector)
        for info in candidates:
            if info["index"] == idx:
                return info
        return None

    selector_lower = selector.lower()
    for info in candidates:
        if selector_lower in info["name"].lower():
            return info
    return None


def find_blackhole_device(devices, require_input=True):
    """Find the first BlackHole-like device or a user-selected override."""
    if SYSTEM_AUDIO_DEVICE:
        return resolve_device(devices, SYSTEM_AUDIO_DEVICE, require_input=require_input)

    candidates = devices
    if require_input:
        candidates = [info for info in devices if info["maxInputChannels"] > 0]

    for info in candidates:
        name = info["name"].lower()
        if any(fragment in name for fragment in BLACKHOLE_FRAGMENTS):
            return info
    return None


def list_input_devices(devices):
    """List all available input devices."""
    print("\n  available input devices:")
    found_any = False
    for info in devices:
        if info["maxInputChannels"] > 0:
            found_any = True
            print(f"    [{info['index']}] {info['name']} ({info['maxInputChannels']}ch)")
    if not found_any:
        print("    [none]")


def print_blackhole_help():
    """Explain how to make system audio capture visible to the recorder."""
    print("  fix:")
    print("    1. Install BlackHole 2ch: brew install --cask blackhole-2ch")
    print("    2. Reboot after the installer finishes so CoreAudio loads the driver.")
    print("    3. Open Audio MIDI Setup and create a Multi-Output Device with BlackHole + your speakers.")
    print("    4. Set macOS output to that Multi-Output Device while recording.")
    print("    5. Re-run the recorder, or set SYSTEM_AUDIO_DEVICE to an exact device name/index.")


def blackhole_driver_installed():
    """Check whether a BlackHole driver bundle exists on disk."""
    return any(os.path.exists(os.path.join(HAL_PLUGIN_DIR, bundle)) for bundle in BLACKHOLE_DRIVER_BUNDLES)


def get_mic_device(pa, devices):
    """Resolve the microphone device, honoring an optional override."""
    if MIC_DEVICE:
        return resolve_device(devices, MIC_DEVICE, require_input=True)

    try:
        default_mic = dict(pa.get_default_input_device_info())
    except OSError:
        return None

    default_mic["index"] = int(default_mic["index"])
    return default_mic


def mix_audio(mic_data, sys_data):
    """Mix two mono int16 audio buffers, clamping to prevent clipping."""
    mic_samples = struct.unpack(f"<{len(mic_data)//2}h", mic_data)
    sys_samples = struct.unpack(f"<{len(sys_data)//2}h", sys_data)
    mixed = []
    for m, s in zip(mic_samples, sys_samples):
        val = m + s
        val = max(-32768, min(32767, val))
        mixed.append(val)
    return struct.pack(f"<{len(mixed)}h", *mixed)


def run_session():
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    wav_file = os.path.join(SESSION_DIR, f"session_{timestamp}.wav")
    audio_chunks = []

    pa = pyaudio.PyAudio()
    devices = get_devices(pa)

    if not devices:
        print("\n[error] PortAudio cannot see any devices.")
        print("  If you are running this from a restricted shell, try launching it from a normal Terminal.")
        pa.terminate()
        return

    # find system audio device for loopback capture
    bh_info = find_blackhole_device(devices, require_input=True)
    any_blackhole = find_blackhole_device(devices, require_input=False)
    if bh_info is None:
        if SYSTEM_AUDIO_DEVICE:
            print(f"\n  [warn] SYSTEM_AUDIO_DEVICE={SYSTEM_AUDIO_DEVICE!r} was not found as an input device")
        elif blackhole_driver_installed():
            print("\n  [warn] BlackHole is installed on disk, but CoreAudio has not loaded it yet")
            print("  reboot macOS, then start the recorder again")
        elif any_blackhole is not None:
            print(f"\n  [warn] Found {any_blackhole['name']}, but it has no input channels")
        else:
            print("\n  [warn] BlackHole not found — recording mic only")
        list_input_devices(devices)
        print_blackhole_help()
    else:
        print(f"\n  system audio: [{bh_info['index']}] {bh_info['name']}")

    # find microphone device
    mic_info = get_mic_device(pa, devices)
    if mic_info is None:
        print("\n[error] No microphone input device is available.")
        list_input_devices(devices)
        pa.terminate()
        return
    print(f"  microphone:   [{mic_info['index']}] {mic_info['name']}")

    stop_event = threading.Event()

    try:
        mic_stream = pa.open(format=FORMAT, channels=CHANNELS, rate=RATE,
                             input=True, input_device_index=mic_info["index"],
                             frames_per_buffer=CHUNK)
    except Exception as exc:
        print(f"[error] could not open microphone [{mic_info['index']}] {mic_info['name']}: {exc}")
        if MIC_DEVICE:
            print("  Try a different MIC_DEVICE value or clear the override.")
        pa.terminate()
        return

    sys_stream = None
    if bh_info is not None:
        try:
            sys_stream = pa.open(format=FORMAT, channels=CHANNELS, rate=RATE,
                                 input=True, input_device_index=bh_info["index"],
                                 frames_per_buffer=CHUNK)
        except Exception as exc:
            print(f"  [warn] could not open system audio [{bh_info['index']}] {bh_info['name']}: {exc}")
            print("  continuing with mic only")
            sys_stream = None

    print(f"\n  recording → {wav_file}")
    print("  press enter to stop\n")

    def _record_audio():
        while not stop_event.is_set():
            try:
                mic_data = mic_stream.read(CHUNK, exception_on_overflow=False)
                if sys_stream:
                    sys_data = sys_stream.read(CHUNK, exception_on_overflow=False)
                    data = mix_audio(mic_data, sys_data)
                else:
                    data = mic_data
                audio_chunks.append(data)
            except Exception:
                break

    audio_thread = threading.Thread(target=_record_audio, daemon=True)
    audio_thread.start()

    input()  # block until enter
    stop_event.set()

    mic_stream.stop_stream()
    mic_stream.close()
    if sys_stream:
        sys_stream.stop_stream()
        sys_stream.close()
    pa.terminate()

    with wave.open(wav_file, "wb") as wf:
        wf.setnchannels(CHANNELS)
        wf.setsampwidth(2)  # 16-bit
        wf.setframerate(RATE)
        wf.writeframes(b"".join(audio_chunks))

    print(f"  saved audio → {wav_file}")

    gemini_file = os.path.join(SESSION_DIR, f"session_{timestamp}_gemini.txt")
    transcribe_with_gemini(wav_file, gemini_file)

    push_session_to_github([wav_file, gemini_file], timestamp)
    print()


def _gemini_upload_file(wav_path):
    """Upload a file to the Gemini File API and return the file URI."""
    file_size = os.path.getsize(wav_path)
    display_name = os.path.basename(wav_path)

    # Step 1: initiate resumable upload
    start_url = f"https://generativelanguage.googleapis.com/upload/v1beta/files?key={GEMINI_API_KEY}"
    start_resp = requests.post(
        start_url,
        headers={
            "X-Goog-Upload-Protocol": "resumable",
            "X-Goog-Upload-Command": "start",
            "X-Goog-Upload-Header-Content-Length": str(file_size),
            "X-Goog-Upload-Header-Content-Type": "audio/wav",
            "Content-Type": "application/json",
        },
        json={"file": {"display_name": display_name}},
        timeout=30,
    )
    start_resp.raise_for_status()
    upload_url = start_resp.headers["X-Goog-Upload-URL"]

    # Step 2: upload the bytes
    with open(wav_path, "rb") as f:
        upload_resp = requests.post(
            upload_url,
            headers={
                "X-Goog-Upload-Offset": "0",
                "X-Goog-Upload-Command": "upload, finalize",
                "Content-Length": str(file_size),
            },
            data=f,
            timeout=300,
        )
    upload_resp.raise_for_status()

    file_uri = upload_resp.json()["file"]["uri"]

    # Step 3: wait for processing
    file_name = upload_resp.json()["file"]["name"]
    check_url = f"https://generativelanguage.googleapis.com/v1beta/{file_name}?key={GEMINI_API_KEY}"
    import time
    for _ in range(60):
        state = requests.get(check_url, timeout=15).json().get("state", "")
        if state == "ACTIVE":
            return file_uri
        if state == "FAILED":
            raise RuntimeError("Gemini file processing failed")
        time.sleep(2)
    raise RuntimeError("Gemini file processing timed out")


def _split_wav(wav_path, chunk_minutes=15):
    """Split a WAV file into chunks and return list of (chunk_path, offset_seconds)."""
    chunk_seconds = chunk_minutes * 60
    chunks = []

    with wave.open(wav_path, "rb") as wf:
        params = wf.getparams()
        total_frames = wf.getnframes()
        rate = wf.getframerate()
        frames_per_chunk = chunk_seconds * rate
        total_duration = total_frames / rate

        if total_duration <= chunk_seconds + 60:  # not worth splitting if barely over
            return [(wav_path, 0)]

        idx = 0
        while wf.tell() < total_frames:
            remaining = total_frames - wf.tell()
            n = min(frames_per_chunk, remaining)
            data = wf.readframes(n)
            offset_s = idx * chunk_seconds

            chunk_path = wav_path.replace(".wav", f"_chunk{idx}.wav")
            with wave.open(chunk_path, "wb") as cf:
                cf.setparams(params)
                cf.writeframes(data)
            chunks.append((chunk_path, offset_s))
            idx += 1

    return chunks


def _transcribe_chunk(audio_part, offset_seconds):
    """Transcribe a single audio chunk via Gemini and return the text."""
    offset_mm = int(offset_seconds // 60)
    offset_ss = int(offset_seconds % 60)
    offset_hint = f"This audio chunk starts at {offset_mm:02d}:{offset_ss:02d} in the full recording. Adjust your timestamps accordingly.\n" if offset_seconds > 0 else ""

    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={GEMINI_API_KEY}"
    payload = {
        "contents": [{
            "parts": [
                {"text": (
                    f"{offset_hint}"
                    "Transcribe this audio verbatim. Do not summarize or paraphrase.\n"
                    "The conversation is primarily in Romanian but may include English words or sentences — "
                    "preserve the original language as spoken.\n"
                    "Identify and label different speakers as [Speaker 1], [Speaker 2], etc.\n"
                    "Include timestamps in MM:SS format at each speaker change.\n"
                    "Output only the transcription, nothing else."
                )},
                audio_part
            ]
        }],
        "generationConfig": {
            "temperature": 0.1
        }
    }

    import time as _time
    for attempt in range(3):
        resp = requests.post(url, json=payload, timeout=300)
        if resp.status_code == 503:
            wait = 10 * (attempt + 1)
            print(f"  [retry] 503, waiting {wait}s...")
            _time.sleep(wait)
            continue
        if resp.status_code != 200:
            print(f"  [error] gemini API returned {resp.status_code}: {resp.text[:200]}")
            return None
        break
    else:
        print(f"  [error] gemini API returned 503 after 3 retries")
        return None

    data = resp.json()
    try:
        return data["candidates"][0]["content"]["parts"][0]["text"]
    except (KeyError, IndexError):
        print(f"  [error] unexpected gemini response")
        return None


def transcribe_with_gemini(wav_path, out_path):
    """Send recorded audio to Gemini Flash for transcription."""
    if not GEMINI_API_KEY:
        print("  [skip] GEMINI_API_KEY not set")
        return

    print("  transcribing with gemini flash...")

    chunks = _split_wav(wav_path)
    print(f"  split into {len(chunks)} chunk(s)")

    all_text = []
    for i, (chunk_path, offset_s) in enumerate(chunks):
        print(f"  processing chunk {i+1}/{len(chunks)} (offset {int(offset_s//60)}m)...")

        file_size = os.path.getsize(chunk_path)
        if file_size > 20 * 1024 * 1024:
            print(f"  uploading {file_size // (1024*1024)}MB via File API...")
            file_uri = _gemini_upload_file(chunk_path)
            audio_part = {"file_data": {"mime_type": "audio/wav", "file_uri": file_uri}}
        else:
            with open(chunk_path, "rb") as f:
                audio_b64 = base64.standard_b64encode(f.read()).decode("utf-8")
            audio_part = {"inline_data": {"mime_type": "audio/wav", "data": audio_b64}}

        text = _transcribe_chunk(audio_part, offset_s)
        if text:
            all_text.append(text)

        # clean up chunk file
        if chunk_path != wav_path:
            os.remove(chunk_path)

    if not all_text:
        print("  [error] no transcription produced")
        return

    with open(out_path, "w") as f:
        f.write("\n\n".join(all_text))

    print(f"  saved gemini transcript → {out_path}")


def push_session_to_github(session_files, timestamp):
    """Clone the GH repo, copy session files into the target folder, commit and push."""
    if not GH_SESSIONS_REPO:
        return

    print("  pushing session to github...")

    tmpdir = tempfile.mkdtemp(prefix="session_push_")
    try:
        env = {**os.environ, "GIT_CLONE_PROTECTION_ACTIVE": "false"}
        subprocess.run(["git", "clone", "--depth", "1", GH_SESSIONS_REPO, tmpdir],
                        check=True, capture_output=True, env=env, timeout=300)

        # Set up Git LFS for large audio files
        subprocess.run(["git", "lfs", "install"], cwd=tmpdir,
                        check=True, capture_output=True)
        subprocess.run(["git", "lfs", "track", "*.wav"], cwd=tmpdir,
                        check=True, capture_output=True)

        dest = os.path.join(tmpdir, GH_SESSIONS_FOLDER)
        os.makedirs(dest, exist_ok=True)

        for src_path in session_files:
            if os.path.exists(src_path):
                shutil.copy2(src_path, dest)

        subprocess.run(["git", "add", ".gitattributes"], cwd=tmpdir, check=True, capture_output=True)
        subprocess.run(["git", "add", "."], cwd=tmpdir, check=True, capture_output=True)

        status = subprocess.run(["git", "status", "--porcelain"], cwd=tmpdir,
                                capture_output=True, text=True)
        if not status.stdout.strip():
            print("  [skip] nothing new to push")
            return

        msg = f"session {timestamp}"
        subprocess.run(["git", "commit", "-m", msg], cwd=tmpdir,
                        check=True, capture_output=True)
        subprocess.run(["git", "push"], cwd=tmpdir, check=True, capture_output=True)

        print(f"  pushed to {GH_SESSIONS_REPO}")
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.decode() if exc.stderr else str(exc)
        print(f"  [error] git push failed: {stderr[:200]}")
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


if __name__ == "__main__":
    print("\n=== audio recorder ===")
    while True:
        cmd = input("\n[s]tart / [q]uit: ").strip().lower()
        if cmd == "s":
            run_session()
        elif cmd == "q":
            break
