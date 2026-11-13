### Security

- **An exposed `--http` server checks bearer tokens with `jsonwebtoken`
  12.0.0.** The new release is stricter in ways that apply to a token check:
  - a token whose header lists a "critical" extension it does not know is
    refused, as RFC 7515 requires;
  - the remaining confusion between a string and a list in `sub`, `iss` and
    `aud` is fixed;
  - empty HMAC keys are refused.

  reflow2 accepts only RSA, RSA-PSS, ECDSA and Ed25519 tokens, never HMAC, and
  still verifies them with `ring` through jsonwebtoken's provider interface; no
  code changed. The build also sheds `base64` 0.22 and `rand_core` 0.6, which
  were in the tree only through the old version. The hold on version 12,
  recorded on 2026-11-13, is removed. **What to do:** nothing. A token signed
  with a header that names an unknown critical extension, which no common
  issuer sends, is now refused where it was accepted before.
