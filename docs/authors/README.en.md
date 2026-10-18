## Writing plugins for the QQ bridge: making your package work in QQ

For third-party authors who want to add features to Miyu in QQ. You write an ordinary Miyu package (an extension, skills, scripts). Once it is installed, she can use it in QQ private chats and QQ groups too.

This page only covers what is different "in QQ" compared with the terminal and the web app. For how to write a package, its manifest and the extension protocol, read the core repository's author guide: [Writing a Miyu package](https://github.com/miyu-agent/miyu/blob/main/docs/authors/README.en.md). Every method's parameters, answers and error codes are in the core's [protocol reference](https://github.com/miyu-agent/miyu/blob/main/docs/reference/README.en.md) (the venue session ones in [`venues.en.md`](https://github.com/miyu-agent/miyu/blob/main/docs/reference/venues.en.md)). Calling the QQ bridge's methods and its hook points are specified in this repository's [`docs/blueprint/extending.md`](../blueprint/extending.md) (Chinese).

Working examples: the memes package [`miyu-memes`](https://github.com/miyu-agent/miyu-memes) (tools, injected hints, sending pictures) and the render package [`miyu-render`](https://github.com/miyu-agent/miyu-render) (a method the bridge calls).

### 1. Things to know first

1. **A QQ conversation is a session too.** Each group has one session and each private chat has one, called venue sessions. In a venue session she talks and calls tools just as in the terminal. The people on the other side are on QQ, and the bridge sends her replies back to QQ.
2. **Venue sessions belong to the system account**, not to the admin. The bridge creates and drives them, and the admin never sees a tool confirmation prompt for them: nobody can confirm anything in a venue session.
3. **Who can talk to her**:
   - Private chats: only the terminal admin and whitelisted members (`onebot.whitelist`). She answers every message in a private chat.
   - Groups: only groups on the whitelist (the venue cards on the bridge's settings page). "Other groups" are off by default. She listens to everything in a group, and the bridge decides whether she answers (an @ mention, a quote of her, a trigger word, the judge's score).
4. **Whether your tool exists in a venue session depends on the `venues` you register it with.** The bridge does not pick tools for you. The core puts each tool into the sessions its `venues` name.

### 2. Making a tool show up in QQ

When your extension registers tools with `provide`, `venues` says which kinds of session get each tool (core author guide, ["Writing an extension": registering tools](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.en.md#registering-tools-provide)):

| Put in `venues` | Shows up in |
|---|---|
| `local` | The admin's own sessions in the terminal and the web app |
| `private` | QQ private chats |
| `group` | QQ groups |

```json
{"name": "meme_send", "description": "Send a sticker as a message of its own.",
 "input_schema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]},
 "access": "read", "venues": ["local", "private", "group"]}
```

**Access categories (`access`) in venue sessions.** Nobody can confirm in a venue session, so:

- `read` and `write` run as usual. `write` is blocked by the core when read-only mode is on.
- `network` and `outbound` need a person to confirm, so they are always refused in venue sessions. Do not register a tool meant for groups with them.
- `venue` is for actions that leave a mark on the platform (recall a message, mute, poke). It runs without asking in venue sessions. In the admin's local sessions it is judged like `outbound`: it runs directly with full access and asks at the workspace level.
- Part of "this turn's reply" does not count as going out. The memes package's `meme_send` is `read` because the sticker is part of her reply this turn, and the bridge sends it with the reply.

**Who is asking her.** `tool.call` carries `by` and `owner` (core author guide, ["Writing an extension": being called](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.en.md#being-called-toolcall)):

- `by` is who sent the message she is answering right now, for example `{"kind": "external", "venue": "qq:group:123", "id": "qq:456", "role": "manager"}`. In a group, `by` changes whenever a new message arrives and she sees it.
- `owner` says whether the core recognizes that person as the owner.
- **The core does not gate people for tools registered as `venue`.** Who may use the tool (only the owner? only the group's managers?) is up to you: check `by.role` and `owner`, and when you refuse, answer with a tool result that says why and set `error` to true.

**Which group it is.** `tool.call` gives you a session id. To learn which group it is and which bot account received it, call the bridge's `chat.venue` (section 7).

### 3. Adding hints to the conversation

An extension that declares `context.inject` (core author guide, ["Writing an extension": more abilities](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.en.md#more-abilities)) is asked once at the start of every turn: `user.append {"session", "venue", "text"?}`. `venue` is `local`, `private` or `group`, so you can decide whether and what to say in QQ. Answer `{"content": [{"kind", "text"}]}` with at most 3 blocks. They go after the message that started the turn, are written to the log and are replayed byte for byte afterwards.

- An answer later than 300 milliseconds counts as nothing for this turn.
- You wrap and escape the text yourself: `text` goes into the context as is.
- State what is true, do not give orders, and write short English sentences. The memes package's hint is `<meme-hint>This reply comes with a sticker.</meme-hint>`, sent with a probability.
- Text sent every turn costs tokens in every group on every turn. Keep it small.

### 4. Sending pictures

Two ways:

1. **A tool result with `sent`** (what the memes package does). Upload the picture with `blob.put` first, then add `sent` to the tool's answer:

   ```json
   {"blocks": [{"type": "text", "text": "Sent."}],
    "sent": [{"blob": "<from blob.put>", "media_type": "image/gif", "width": 240, "height": 240, "alt": "A cat waving.", "sticker": true}]}
   ```

   At most 4 pictures. `sent` is not shown to the model (she only sees `blocks`). The bridge sends each picture to QQ as one image message, queued in the order she made her calls, together with her words. `sticker: true` is sent as a QQ sticker (OneBot image segment with `sub_type=1`). When a picture cannot be sent (she is muted and it expired in the queue), the bridge writes an `unshown` fact for her so she knows. `path` is not accepted: use `blob.put`.

2. **She calls the core's `show`** to show a file from the workspace. Images go out as pictures, other files as QQ file uploads. Your tool only writes the file into the workspace and returns its path, and she decides whether to show it (the render package's `render_image` works this way).

### 5. Groups versus private chats

| | Private chat | Group |
|---|---|---|
| Who can talk | The terminal admin, whitelisted members | Everyone in a whitelisted group |
| What she answers | Every message | Messages the bridge decides on (@, quote, trigger word, judge score) |
| `by` | The other person | Whoever sent the message she is answering now; it changes |
| Your tool is available when | `venues` includes `private` | `venues` includes `group` |
| The session belongs to | The system account | The system account |

Groups have many people and many strangers. A tool that changes anything must check `by` and `owner`. "She asked me to" does not mean the owner asked.

### 6. The system account and permissions

- **Workspace.** A venue session can only touch its own workspace (the core's file tools check real paths). Your extension process is **not** under that limit: it is your program, and the core cannot stop what it reads or writes. So never open a path from `tool.call` directly. Resolve it to its real location (with links resolved) and require it to be inside `cwd`, the turn's working directory. When writing a file, write a newly created temporary file and rename it, so a link planted beforehand cannot redirect your write. The render package's `render_image` is the model to follow (`src/tools/render_image.rs`, `src/tools/saved.rs`).
- **Untrusted text.** People in a group can write anything, including their nicknames and group cards. If you put such text into a prompt for the model, escape it and wrap it, so nobody can forge a line that looks like a system message.
- **Blobs.** Pictures you `blob.put` are stored under the admin. The bridge can read the ones you hand over in a tool result's `sent` or in a method's answer to it (the core is tightening this rule).
- **Refused by the core.** Methods meant only for local people, such as `extension.*` and `package.install`, are refused for extensions (`local_only`).

### 7. Working with the QQ bridge

Your extension can call the bridge, and the bridge can call you. Both go through the core's `package.call` (core author guide, ["Writing an extension": more abilities](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.en.md#more-abilities)). The manifest must declare `packages.call` and a person must approve it.

**You call the bridge** (details in [`extending.md` section 1](../blueprint/extending.md)):

- Hook points and methods are named `chat.*` since 2026-10-19 (they used to be `qq.*`). Every parameter object the bridge hands you, and the answer of `chat.venue`, carries `platform` (only `qq` for now). When you call a method you may pass `platform`: left out means `qq`; any other platform is refused with `bad_params`.
- `chat.send {user | group, text}`: send a message in her name. It goes through the outbound chain, and the receiving venue's session gets a fact, so she knows she started the conversation.
- `chat.action {action, params?, bot?}`: run a OneBot action for you (kick, list group members, fetch an image…). NapCat's answer comes back unchanged. Messages sent this way are not known to her as her own words.
- `chat.venue {session}`: which group and which bot account a session id belongs to.

**The bridge calls you**:

- **Hook points** (details in [`extending.md` section 2](../blueprint/extending.md)): export methods with the agreed names and they are hooked up once your package is installed and running. `chat.event` (QQ notices and requests, passed on as is), `chat.inbound` (asked before a group message is decided: pass, record only, reply), `chat.outbound` (asked before her words go out: replace, drop, keep), `chat.score` (asked after the judge has scored: add or subtract). `chat.event` does not wait for your answer. The other three wait at most 1.5 seconds and treat no answer as no opinion.
- **Known services**: package ids and method names the bridge knows and uses when installed. There is one now: `miyu-render`'s `markdown {markdown}` → `{pages: [{blob, width, height}]}`. The bridge hands long replies over the threshold to it and sends them as pictures (the "long text" field on each venue card). A package with the same id and method replaces how it is drawn. When the call fails or takes longer than 15 seconds, the bridge splits the text into messages instead.

### 8. Trying it in QQ

1. Get the bridge running: install NapCat and log in the bot account, run `miyu onebot start`, open the settings page with `miyu onebot web`, put its token into NapCat, and set the terminal admin and whitelisted members there. `miyu onebot status` tells you whether it is connected.
2. Install your package: `miyu pkg install ./your-package`, answer "turn it on" and approve the capabilities (core author guide, ["Install, try, check"](https://github.com/miyu-agent/miyu/blob/main/docs/authors/README.en.md#install-try-check)). Venue sessions follow package changes: sessions already open get your tools on their next turn.
3. Send a private message to the bot account from the terminal admin's QQ account and ask her to use your tool. To try a group, add a group to the whitelist on the settings page, turn it on and @ her.
4. See what happened:
   - `miyu onebot logs -f`: the bridge's log (decisions, queueing, whether messages went out, whether hooks answered).
   - `<data root>/state/logs/<your package>.stderr`: your program's standard error.
   - `miyu onebot venue show qq:group:<group number>`: every setting of that group with its value and where it came from.
5. When the preset has your package turned off, venue sessions do not have your tools. Check which persona and preset the venue card uses.

### 9. Manifest

```toml
[package]
protocol = [1, 1]
name = { en = "Hello", zh = "你好" }

[command]
name = "hello"
program = "hello"
about = { en = "Hello package", zh = "你好" }

[process]
args = ["serve"]
start = "always"
capabilities = ["tools", "context.inject", "packages.call"]   # only what you use

# Only if you hook into the bridge:
[depends]
packages = ["onebot"]

[export]
methods = ["chat.inbound"]
```
