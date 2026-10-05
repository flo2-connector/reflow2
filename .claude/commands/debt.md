---
description: What the reflow2 coherence loop owes right now
---
Call reflow2's `loop_status` tool and tell me, as one short list, what the coherence loop currently owes:
open gaps never put to me, questions waiting on me, structural defects, capabilities claiming built with no check, and any drift awaiting a decision.

Read its `next` list as well as `clean`: `clean` is about the coherence loop, and `next` also carries what the design owes beyond it. A line saying the design has NEVER BEEN EXPORTED goes first, because the store is then its only copy. If it reports clean and `next` is empty, just say "nothing owed." Keep it to plain language — this is the quick "what does reflow2 want from me right now?" check.
