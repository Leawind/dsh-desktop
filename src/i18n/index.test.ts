import { describe, expect, it } from "vitest";

import { fallbackLocale, localeMessages, supportedLocales } from "./index";

type MessageTree = { readonly [key: string]: string | MessageTree };

function flattenMessages(tree: MessageTree, prefix = ""): Map<string, string> {
  const messages = new Map<string, string>();
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") messages.set(path, value);
    else for (const entry of flattenMessages(value, path)) messages.set(...entry);
  }
  return messages;
}

function parameters(message: string): Set<string> {
  const values = [...message.matchAll(/\{([a-zA-Z][a-zA-Z0-9]*)\}/g)]
    .map((match) => match[1])
    .filter((value): value is string => value !== undefined);
  return new Set(values);
}

describe("locale resources", () => {
  it("uses zh-CN as the fallback locale", () => {
    expect(fallbackLocale).toBe("zh-CN");
    expect(supportedLocales).toEqual(["zh-CN", "en-US"]);
  });

  it("keeps keys and interpolation parameters aligned", () => {
    const fallback = flattenMessages(localeMessages[fallbackLocale]);
    for (const locale of supportedLocales) {
      const messages = flattenMessages(localeMessages[locale]);
      expect(new Set(messages.keys())).toEqual(new Set(fallback.keys()));
      for (const [key, fallbackMessage] of fallback) {
        expect(parameters(messages.get(key) ?? ""), key).toEqual(parameters(fallbackMessage));
      }
    }
  });
});
