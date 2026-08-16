import assert from "node:assert/strict";
import { generateKeyPairSync, verify } from "node:crypto";
import { describe, it } from "node:test";

import { signManifest } from "./generate-compatibility.js";

describe("compatibility manifest generation", () => {
  it("creates an Ed25519 detached signature", () => {
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");
    const manifest = Buffer.from('{"schemaVersion":1}\n');
    const signature = Buffer.from(
      signManifest(manifest, privateKey.export({ type: "pkcs8", format: "pem" }).toString()),
      "base64",
    );
    assert.equal(verify(null, manifest, publicKey, signature), true);
  });
});
