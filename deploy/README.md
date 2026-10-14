# deploy/ — run the CI-built desktop binary

This folder holds the `computer-agent` binary built by CI
(`release.yml`, `--release`, Ubuntu). The binary itself is git-ignored;
only this README and `fetch-binary.sh` are committed.

## 1. Download

```bash
cd deploy
./fetch-binary.sh          # latest green v0.3.0 build
./fetch-binary.sh v0.4.0   # a specific tag
```

Needs a token with repo read access: `COMPUTERAGENT_TOKEN` env var,
or credentials already stored for `github.com` via `git credential`.
Manual alternative: GitHub → Pilser/computerAgent → Actions → pick the
green run → download the `computerAgent-<tag>` artifact, unzip here.

## 2. System deps (Linux)

```bash
sudo apt install libfontconfig1
```

(Most desktops already have it. Wayland/X11 client libs ship with the
desktop; no dev packages needed to *run* the binary.)

## 3. Configure the model

The embedded engine calls an OpenAI-compatible API. Set one key before
launching (Ollama needs no key for local models):

```bash
export OPENAI_API_KEY="sk-..."        # or ANTHROPIC_API_KEY / GEMINI_API_KEY /
                                      # OPENROUTER_API_KEY / OLLAMA_API_KEY
./computer-agent
```

Model choice lives in the app (model picker, 12-model catalog with
context windows + per-1M pricing). Defaults to the first catalog entry.

## 4. Data

Sessions, memory (`MEMORY.md`, daily logs, notes) and chat history live
under `$XDG_DATA_HOME/computer-agent/` (`~/.local/share/computer-agent/`
unless `CA_DATA_DIR` is set). Back that folder up to keep your threads.

## 5. Test checklist

- [ ] Window opens with the warm welcome hero, no console errors.
- [ ] New thread → send "hi" → stub turn answers, tokens + cost update.
- [ ] History lists the thread; reopen it; delete works.
- [ ] Model picker switches model; token bar + cost follow the catalog.
- [ ] ` ```mermaid ` fence renders a diagram; bad syntax falls back to code.
- [ ] Restart keeps threads (disk persistence).

## 6. Troubleshooting

- `error while loading shared libraries: libfontconfig.so.1` → install it (§2).
- Blank window on Wayland → `QT_QPA_PLATFORM=xcb ./computer-agent`.
- No answer, "provider" error → key missing/wrong, check §3.
- Wipe local state → stop the app, `rm -rf ~/.local/share/computer-agent`.
