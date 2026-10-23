# From the tester journal to product diagnostics

Status: a plan, plus the first piece (crash capture). Only the tester program
exists today: it is limited to enrolled accounts, shows the tester every entry
it records, and sends reports to the founder's own chat. Nothing below that is
marked "planned" is built.

## Where it starts and where it should end

| Stage | Who | What leaves the machine | State |
| --- | --- | --- | --- |
| 1. Tester program | up to 3 enrolled accounts | the journal, notes and pictures the tester chooses to send | built |
| 1b. Crash capture | the same testers | a note that the last run ended abnormally, and the panic message and source line if there was one | built in this change |
| 2. Opt-in diagnostics | any user who turns it on | the same crash note and a small counter set, no journal, no pictures, no account | planned |
| 3. Self-sufficient program | all users, after a clear first-run choice | stage 2 data, plus an explicit "send a report" button that works like the tester report | planned |

## Rules every stage keeps

- **Visible.** Anything recorded can be seen in the app before it is sent. No
  hidden capture, no screen recording, no clipboard, no typed text.
- **Minimal.** Error codes, app version, Windows version, which screen, how long
  an engine step took. Never a path, a Steam or Dota account, a hero or item
  choice tied to a person, an email, or an IP address kept after rate limiting.
  The crash note already replaces the home folder and `Users\<name>` with `~`
  and bounds every text.
- **Consent first.** Stages 2 and 3 are off until the person chooses, with the
  choice and what it covers on screen, and a switch in Settings that works
  offline. The choice is remembered per install, not per account.
- **Anonymous by default.** A random install ID that is not linked to an
  account, resettable in Settings. A "delete what you sent" action removes
  everything stored for that ID.
- **Bounded.** Size, count and rate limits on the Worker; retention of 30 days
  for raw events, counters kept longer; a kill switch on the Worker that turns
  collection off for everyone without an app update.
- **Honest in the privacy page.** `website/public/privacy.html` is updated
  before stage 2 ships, not after.

## What stage 2 would add (planned)

1. An engine event stream limited to codes: which step failed, the error code,
   the duration. The builder already produces these; the tester recorder
   already subscribes to them.
2. A crash/abnormal-exit uploader reading the note from `take_previous_run`
   (built) and posting it to a Worker endpoint that keeps counters in D1 and
   forwards nothing else.
3. A daily digest to the founder's chat: top error codes by version, installs
   that ended abnormally, steps that got slower. Done by the existing bot and
   the Worker's scheduled trigger.
4. An in-app "something went wrong" card that offers to send one report, with
   the same preview the tester window has.
5. Release gating: the digest shows whether a new EA build raised the rate of a
   code, so a bad build is noticed from numbers, not from messages.

## What stage 1b does today

On every start the app writes `session-alive.json`; a normal exit removes it. If
it is still there at the next start, or `last-panic.json` exists, the tester
window offers to describe and send that. The note holds the time, the version
and, for a panic, the message and the source file and line (home folder and user
name removed). The Worker validates it, drops it if it is malformed, and puts
one line at the top of the Telegram summary. It does not detect a crash inside
the web view that leaves the process running, or a freeze; those need the
interface side (planned).
