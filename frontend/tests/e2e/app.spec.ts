import { expect, test } from "@playwright/test";
import {
  commandCalls,
  installTauriMock,
  makeMockState,
  mockSession,
  setMockState,
} from "./mockTauri";

test("first-run setup fixes Gemini and sessions folder from the main screen", async ({
  page,
}) => {
  await installTauriMock(
    page,
    makeMockState({
      hasGeminiKey: false,
      settings: { sessionsDir: null },
      selectFolderResult: null,
    })
  );

  await page.goto("/");

  await expect(
    page.getByRole("banner").getByText("Missing: Gemini key, sessions folder")
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Gemini API Key" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Record" })).toBeDisabled();

  await expect(page.getByRole("button", { name: "Save key" })).toBeDisabled();
  await page.getByLabel("Paste key").fill("   ");
  await page.getByRole("button", { name: "Save key" }).click();
  await expect(page.getByText("Paste a key first.")).toBeVisible();

  await page.getByLabel("Paste key").fill("AIzaSy_test_key");
  await page.getByRole("button", { name: "Reveal key" }).click();
  await expect(page.getByLabel("Paste key")).toHaveAttribute("type", "text");
  await page.getByRole("button", { name: "Hide key" }).click();
  await page.getByRole("button", { name: "Save key" }).click();
  await expect(page.getByText("Key saved and validated.")).toBeVisible();

  await setMockState(page, { selectFolderResult: "/tmp/reef-recorder-ready" });
  await page.getByRole("button", { name: /Folder/ }).click();
  await expect(page.getByRole("heading", { name: "Sessions Folder" })).toBeVisible();
  await page.getByRole("button", { name: "Choose folder" }).click();

  await expect(page.getByRole("banner").getByText("Ready to record")).toBeVisible();
  await expect(page.getByRole("button", { name: "Record" })).toBeEnabled();

  const calls = await commandCalls(page);
  expect(calls.some((call) => call.cmd === "save_gemini_key")).toBe(true);
  expect(calls.filter((call) => call.cmd === "select_sessions_folder")).toHaveLength(2);
});

test("Gemini panel validates typed and stored keys, then removes saved key", async ({
  page,
}) => {
  await installTauriMock(page);
  await page.goto("/");

  await page.getByRole("button", { name: /Gemini/ }).click();
  await expect(page.getByRole("heading", { name: "Gemini API Key" })).toBeVisible();

  await page.getByLabel("Replace key").fill("bad-key");
  await page.getByRole("button", { name: "Validate" }).click();
  await expect(page.getByText("Validation failed: invalid Gemini key.")).toBeVisible();

  await page.getByLabel("Replace key").fill("");
  await page.getByRole("button", { name: "Validate" }).click();
  await expect(page.getByText("Validation complete.")).toBeVisible();

  page.on("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Remove" }).click();
  await expect(page.getByText("Key removed.")).toBeVisible();
  await expect(page.getByRole("banner").getByText("Missing: Gemini key")).toBeVisible();
});

test("microphone and system-audio setup panels show device and permission actions", async ({
  page,
}) => {
  await installTauriMock(
    page,
    makeMockState({
      micStatus: { state: "denied", detail: "Microphone access denied." },
      settings: { captureSystemAudio: true },
      devices: [
        {
          id: "mic-1",
          name: "Studio Mic",
          inputChannels: 2,
          isDefault: true,
          isBlackhole: false,
        },
        {
          id: "blackhole-2ch",
          name: "BlackHole 2ch",
          inputChannels: 2,
          isDefault: false,
          isBlackhole: true,
        },
      ],
    })
  );

  await page.goto("/");

  await page.getByRole("button", { name: /Mic/ }).click();
  await expect(page.getByText("Microphone access is denied.")).toBeVisible();
  await expect(page.getByText("Studio Mic")).toBeVisible();
  await page.getByRole("button", { name: "Open microphone settings" }).click();

  await page.getByRole("button", { name: /System Audio/ }).click();
  await expect(
    page.getByRole("listitem").filter({ hasText: "BlackHole 2ch" })
  ).toBeVisible();
  await page.getByText("Setup checklist").click();
  await expect(page.getByText("brew install --cask blackhole-2ch")).toBeVisible();
  await page.getByRole("button", { name: "Open sound settings" }).click();
  await page.getByRole("button", { name: "Audio MIDI Setup" }).click();

  const calls = await commandCalls(page);
  expect(
    calls.filter((call) => call.cmd === "open_permissions_settings").map((call) => call.args.kind)
  ).toEqual(["microphone", "soundSettings", "audioMidiSetup"]);
});

test("settings sheet edits audio, Gemini, cost, and GitHub settings", async ({
  page,
}) => {
  await installTauriMock(page);
  await page.goto("/");

  await page.getByRole("button", { name: "Open settings" }).click();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Save changes" })).toBeDisabled();

  await page
    .getByLabel("Capture system audio when BlackHole is available")
    .check();
  await page.getByLabel("Microphone selector").fill("USB Mic");
  await page.getByLabel("System audio selector").fill("BlackHole");
  await page
    .getByRole("textbox", { name: "Model", exact: true })
    .fill("gemini-test-primary");
  await page.getByLabel("Fallback model").fill("gemini-test-fallback");
  await page.getByLabel("Chunk minutes").fill("3");
  await page.getByLabel("Language hint").fill("Romanian and English");
  await page.getByLabel("Label speakers as [Speaker 1], [Speaker 2]").uncheck();
  await page.getByLabel("Include [MM:SS] timestamps").uncheck();
  await page.getByLabel("Input $/1M tokens").fill("1.25");
  await page.getByLabel("Output $/1M tokens").fill("4.5");
  await page.getByRole("dialog").getByLabel("Enable GitHub sync").check();
  await page.getByLabel("Repository URL").fill("git@github.com:org/repo.git");
  await page.getByLabel("Target folder").fill("meetings");
  await page.getByLabel("Use Git LFS for WAV files").uncheck();
  await page.getByRole("button", { name: "Save changes" }).click();

  await expect(page.getByText("Saved.")).toBeVisible();
  const calls = await commandCalls(page);
  const saveCall = calls.findLast((call) => call.cmd === "save_settings");
  expect(saveCall?.args.input).toMatchObject({
    micDeviceSelector: "USB Mic",
    systemAudioDeviceSelector: "BlackHole",
    geminiModel: "gemini-test-primary",
    geminiFallbackModel: "gemini-test-fallback",
    chunkMinutes: 3,
    languageHint: "Romanian and English",
    includeSpeakerLabels: false,
    includeTimestamps: false,
    geminiInputCostPerMillionUsd: 1.25,
    geminiOutputCostPerMillionUsd: 4.5,
    githubSyncEnabled: true,
    githubRepoUrl: "git@github.com:org/repo.git",
    githubTargetFolder: "meetings",
    gitLfsEnabled: false,
  });
});

test("GitHub setup validates incomplete and ready sync settings", async ({
  page,
}) => {
  await installTauriMock(
    page,
    makeMockState({
      gitLfsInstalled: false,
      settings: {
        githubSyncEnabled: true,
        githubRepoUrl: "",
        githubTargetFolder: "../bad",
        gitLfsEnabled: true,
      },
    })
  );

  await page.goto("/");
  await page.getByRole("button", { name: /GitHub/ }).click();

  await expect(page.getByText("git-lfs installed: no")).toBeVisible();
  await expect(page.getByText("repo URL set: no")).toBeVisible();
  await expect(page.getByText("target folder safe: no")).toBeVisible();
  await page.getByRole("button", { name: "Test sync tools" }).click();
  await expect(page.getByText("git-lfs not installed")).toBeVisible();

  await setMockState(page, { gitLfsInstalled: true });
  await page.getByLabel("Repository URL").fill("git@github.com:org/repo.git");
  await page.getByLabel("Target folder").fill("sessions");
  await page.getByRole("button", { name: "Save" }).click();
  await page.getByRole("button", { name: "Test sync tools" }).click();
  await expect(page.getByText("Local tooling is ready.")).toBeVisible();
});

test("recording stop saves a session and transcribes it", async ({ page }) => {
  await installTauriMock(page);
  await page.goto("/");

  await page.getByRole("button", { name: "Record" }).click();
  await expect(page.getByRole("button", { name: "Stop" })).toBeVisible();
  await page.waitForTimeout(1100);
  await expect(page.locator(".recorder-timer")).not.toHaveText("00:00");

  await page.getByRole("button", { name: "Stop" }).click();
  const row = page.locator(".session-row").filter({
    hasText: "session_20260511_101500",
  });
  await expect(row).toBeVisible();
  await expect(row.getByText("transcript")).toBeVisible();

  await row.click();
  await expect(
    page.getByRole("dialog", {
      name: "Transcript for session_20260511_101500",
    })
  ).toBeVisible();
  await expect(
    page.getByText("[00:00] [Speaker 1] We covered the complete Reef Recorder workflow.")
  ).toBeVisible();
});

test("recording start and transcription failures remain recoverable", async ({
  page,
}) => {
  await installTauriMock(
    page,
    makeMockState({
      commandFailures: {
        start_recording: {
          message: "microphone: permission denied",
          once: true,
        },
      },
      transcribeFailureOnce: true,
    })
  );
  await page.goto("/");

  await page.getByRole("button", { name: "Record" }).click();
  await expect(page.getByText("microphone: permission denied")).toBeVisible();

  await page.getByRole("button", { name: "Record" }).click();
  await page.getByRole("button", { name: "Stop" }).click();
  await expect(page.getByText("transcript failed")).toBeVisible();

  await expect(page.getByText("Gemini rate limit.")).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Retry transcription" })
    .click();
  await expect(page.getByText("Copy transcript")).toBeVisible();
});

test("session row supports retranscribe, sync failure retry, clear WAV, and delete", async ({
  page,
}) => {
  await installTauriMock(
    page,
    makeMockState({
      settings: {
        githubSyncEnabled: true,
        githubRepoUrl: "git@github.com:org/repo.git",
      },
      sessions: [
        mockSession("session_20260511_111100", {
          syncStatus: "failed",
          syncError: "Push rejected.",
        }),
      ],
      syncFailureOnce: false,
    })
  );
  page.on("dialog", (dialog) => dialog.accept());
  await page.goto("/");

  const row = page.locator(".session-row").filter({
    hasText: "session_20260511_111100",
  });
  await expect(row.getByText("sync failed")).toBeVisible();
  await row.getByRole("button", { name: "Retranscribe" }).click();
  await expect(row.getByText("transcript")).toBeVisible();

  await row.getByRole("button", { name: "Retry sync" }).click();
  await expect(row.getByText("synced")).toBeVisible();

  await row.getByRole("button", { name: "Clear WAV" }).click();
  await expect(row.getByRole("button", { name: "Clear WAV" })).toBeDisabled();

  await row.getByRole("button", { name: "Delete session" }).click();
  await expect(page.getByText("No sessions yet. Record to create one.")).toBeVisible();
});

test("transcript drawer copies, reveals files, syncs, and closes", async ({
  page,
  context,
}) => {
  await page.setViewportSize({ width: 900, height: 720 });
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await installTauriMock(
    page,
    makeMockState({
      settings: { githubSyncEnabled: true },
      sessions: [
        mockSession("session_20260511_121200", {
          syncStatus: "skipped",
        }),
      ],
      transcripts: {
        session_20260511_121200:
          "[00:00] [Speaker 1] Clipboard transcript text.",
      },
    })
  );
  await page.goto("/");

  await page.getByText("session_20260511_121200").click();
  await page.getByRole("button", { name: "Copy transcript" }).click();
  await expect(page.getByText("Copied.")).toBeVisible();
  await expect(page.evaluate(() => navigator.clipboard.readText())).resolves.toBe(
    "[00:00] [Speaker 1] Clipboard transcript text."
  );

  await page.getByRole("button", { name: "Reveal WAV" }).click();
  await page.getByRole("button", { name: "Reveal transcript" }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Sync now" })
    .click();
  await page.locator(".transcript-drawer-close").click();
  await expect(page.getByRole("dialog")).toBeHidden();

  const calls = await commandCalls(page);
  expect(calls.filter((call) => call.cmd === "reveal_path")).toHaveLength(2);
  expect(calls.some((call) => call.cmd === "sync_session")).toBe(true);
});

test("bulk session actions clear WAV files and delete all sessions", async ({
  page,
}) => {
  await installTauriMock(
    page,
    makeMockState({
      sessions: [
        mockSession("session_20260511_130000"),
        mockSession("session_20260511_131000"),
      ],
    })
  );
  page.on("dialog", (dialog) => dialog.accept());
  await page.goto("/");

  await page.getByRole("button", { name: "Clear all WAVs" }).click();
  const firstRow = page.locator(".session-row").first();
  await expect(firstRow.getByRole("button", { name: "Clear WAV" })).toBeDisabled();

  await page.getByRole("button", { name: "Delete all sessions" }).click();
  await expect(page.getByText("No sessions yet. Record to create one.")).toBeVisible();
});
