import { createHash } from "node:crypto";
import type { AccountField, AccountStruct } from "./index.js";
import { sizeOfRustType } from "./sizes.js";

const ACCOUNT_DISCRIMINATOR_BYTES = 8;

type StructBlock = {
  name: string;
  body: string;
  endIndex: number;
};

export class AnalysisError extends Error {
  constructor(
    message: string,
    readonly account: string,
    readonly field: string,
    readonly type: string,
    readonly filePath: string
  ) {
    super(message);
    this.name = "AnalysisError";
  }
}

export type RawStruct = {
  name: string;
  fields: Array<{ name: string; type: string }>;
  filePath: string;
};

export function parseAllRawStructs(source: string, filePath: string): RawStruct[] {
  const cleanSource = stripRustComments(source);
  const rawStructs: RawStruct[] = [];
  const structMatch = /\bstruct\s+([A-Za-z_][A-Za-z0-9_]*)\b/g;
  
  let match;
  while ((match = structMatch.exec(cleanSource)) !== null) {
    const structStart = match.index;
    const bodyStart = cleanSource.indexOf("{", structStart);
    if (bodyStart === -1) {
      continue;
    }

    const textBetween = cleanSource.slice(structStart, bodyStart);
    if (textBetween.includes(";") || /\bstruct\b/.test(textBetween.slice(match[0].length))) {
      structMatch.lastIndex = structStart + match[0].length;
      continue;
    }

    const bodyEnd = findMatchingBrace(cleanSource, bodyStart);
    if (bodyEnd === -1) {
      continue;
    }

    const body = cleanSource.slice(bodyStart + 1, bodyEnd);
    const fields: Array<{ name: string; type: string }> = [];

    for (const fieldSource of splitTopLevel(body, ",")) {
      const cleanFieldText = stripMacroAttributes(fieldSource).cleanText.trim();
      if (!cleanFieldText) {
        continue;
      }

      const fieldMatch = /^(?:pub(?:\([^)]+\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$/.exec(cleanFieldText);
      if (fieldMatch) {
        fields.push({
          name: fieldMatch[1],
          type: normalizeRustType(fieldMatch[2])
        });
      }
    }

    rawStructs.push({
      name: match[1],
      fields,
      filePath
    });
  }

  return rawStructs;
}

export function parseAccountStructs(
  source: string,
  filePath: string,
  typesRegistry?: Map<string, RawStruct>
): AccountStruct[] {
  const cleanSource = stripRustComments(source);
  const accountStructs: AccountStruct[] = [];
  let searchIndex = 0;

  while (searchIndex < cleanSource.length) {
    const accountAttributeIndex = cleanSource.indexOf("#[account", searchIndex);

    if (accountAttributeIndex === -1) {
      break;
    }

    const nextStructIndex = cleanSource.indexOf("struct ", accountAttributeIndex);
    if (nextStructIndex === -1) {
      searchIndex = accountAttributeIndex + "#[account".length;
      continue;
    }
    const textBetween = cleanSource.slice(accountAttributeIndex, nextStructIndex);
    if (textBetween.includes("{") || textBetween.includes("}") || textBetween.includes(";")) {
      searchIndex = accountAttributeIndex + "#[account".length;
      continue;
    }

    const structBlock = findNextStructBlock(cleanSource, accountAttributeIndex);

    if (!structBlock) {
      searchIndex = accountAttributeIndex + "#[account".length;
      continue;
    }

    const fields = parseNamedFields(structBlock.body, structBlock.name, filePath, typesRegistry);
    const fieldBytes = fields.reduce((sum, field) => sum + field.byteSize, 0);
    const layoutWarnings = fields
      .filter((field) => field.dynamic)
      .map((field) => ({
        code: "DYNAMIC_TYPE" as const,
        message: "Dynamic size detected. Static realloc analysis may be inaccurate.",
        account: structBlock.name,
        field: field.name,
        type: field.type,
        filePath
      }));

    accountStructs.push({
      accountId: `${filePath}::${structBlock.name}`,
      name: structBlock.name,
      namespace: filePath,
      byteSize: ACCOUNT_DISCRIMINATOR_BYTES + fieldBytes,
      byteSizeIncludesDiscriminator: true,
      abiFingerprint: abiFingerprint(structBlock.name, fields),
      hasDynamicSize: layoutWarnings.length > 0,
      layoutWarnings,
      fields,
      filePath,
      discriminator: computeAccountDiscriminator(structBlock.name)
    });

    searchIndex = structBlock.endIndex + 1;
  }

  return accountStructs;
}

function findNextStructBlock(source: string, fromIndex: number): StructBlock | null {
  const structMatch = /\bstruct\s+([A-Za-z_][A-Za-z0-9_]*)\b/g;
  structMatch.lastIndex = fromIndex;
  
  while (true) {
    const match = structMatch.exec(source);
    if (!match || match.index === undefined) {
      return null;
    }

    const structStart = match.index;
    const bodyStart = source.indexOf("{", structStart);
    if (bodyStart === -1) {
      return null;
    }

    const textBetween = source.slice(structStart, bodyStart);
    if (textBetween.includes(";") || /\bstruct\b/.test(textBetween.slice(match[0].length))) {
      structMatch.lastIndex = structStart + match[0].length;
      continue;
    }

    const bodyEnd = findMatchingBrace(source, bodyStart);
    if (bodyEnd === -1) {
      return null;
    }

    return {
      name: match[1],
      body: source.slice(bodyStart + 1, bodyEnd),
      endIndex: bodyEnd
    };
  }
}

function findMatchingBrace(source: string, openBraceIndex: number): number {
  let depth = 0;

  for (let index = openBraceIndex; index < source.length; index += 1) {
    const character = source[index];

    if (character === "{") {
      depth += 1;
    } else if (character === "}") {
      depth -= 1;

      if (depth === 0) {
        return index;
      }
    }
  }

  return -1;
}

function stripMacroAttributes(text: string): { cleanText: string; warnings: string[] } {
  let output = "";
  let index = 0;
  const warnings: string[] = [];

  while (index < text.length) {
    if (text[index] === "#" && text[index + 1] === "[") {
      const macroStart = index;
      index += 2;
      let depth = 1;
      let closed = false;

      while (index < text.length) {
        const char = text[index];
        if (char === "[") {
          depth += 1;
        } else if (char === "]") {
          depth -= 1;
          if (depth === 0) {
            closed = true;
            index += 1;
            break;
          }
        }
        index += 1;
      }

      if (!closed) {
        warnings.push(`Unclosed macro attribute starting at index ${macroStart}`);
      }
      continue;
    }

    output += text[index];
    index += 1;
  }

  return { cleanText: output, warnings };
}

function parseNamedFields(
  body: string,
  accountName: string,
  filePath: string,
  typesRegistry?: Map<string, RawStruct>
): AccountField[] {
  const fields: AccountField[] = [];

  for (const fieldSource of splitTopLevel(body, ",")) {
    const { cleanText, warnings } = stripMacroAttributes(fieldSource);
    if (warnings.length > 0) {
      console.warn(`⚠️ Parser warnings in ${filePath} (account ${accountName}):\n` + warnings.join("\n"));
    }

    const cleanFieldText = cleanText.trim();
    if (!cleanFieldText) {
      continue;
    }

    const fieldMatch = /^(?:pub(?:\([^)]+\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$/.exec(cleanFieldText);

    if (!fieldMatch) {
      throw new Error(`EPIC Parser: Unable to parse field definition "${cleanFieldText}" in account struct "${accountName}" at ${filePath}.`);
    }

    const type = normalizeRustType(fieldMatch[2]);
    const sized = sizeOfRustType(type, typesRegistry);

    if (sized.byteSize === null) {
      throw new AnalysisError(
        [
          `Unable to resolve type "${type}"`,
          `Account: ${accountName}`,
          `Field: ${fieldMatch[1]}`,
          `File: ${filePath}`,
          "",
          "EPIC cannot safely calculate layout size.",
          "",
          "Analysis aborted."
        ].join("\n"),
        accountName,
        fieldMatch[1],
        type,
        filePath
      );
    }

    fields.push({
      name: fieldMatch[1],
      type,
      byteSize: sized.byteSize,
      dynamic: sized.dynamic,
      ...(sized.note ? { note: sized.note } : {})
    });
  }

  return fields;
}

function splitTopLevel(source: string, delimiter: string): string[] {
  const parts: string[] = [];
  let startIndex = 0;
  let angleDepth = 0;
  let bracketDepth = 0;
  let parenDepth = 0;

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];

    if (character === "<") {
      angleDepth += 1;
    } else if (character === ">") {
      angleDepth = Math.max(0, angleDepth - 1);
    } else if (character === "[") {
      bracketDepth += 1;
    } else if (character === "]") {
      bracketDepth = Math.max(0, bracketDepth - 1);
    } else if (character === "(") {
      parenDepth += 1;
    } else if (character === ")") {
      parenDepth = Math.max(0, parenDepth - 1);
    } else if (
      character === delimiter &&
      angleDepth === 0 &&
      bracketDepth === 0 &&
      parenDepth === 0
    ) {
      parts.push(source.slice(startIndex, index));
      startIndex = index + 1;
    }
  }

  parts.push(source.slice(startIndex));
  return parts;
}

function normalizeRustType(type: string): string {
  return type.replace(/\s+/g, " ").trim();
}

function abiFingerprint(accountName: string, fields: AccountField[]): string {
  const input = JSON.stringify({
    account: accountName,
    fields: fields.map((field) => ({ name: field.name, type: field.type }))
  });

  return createHash("sha256").update(input).digest("hex");
}

function computeAccountDiscriminator(structName: string): string {
  const hash = createHash("sha256").update(`account:${structName}`).digest();
  return "0x" + hash.subarray(0, 8).toString("hex");
}

function stripRustComments(source: string): string {
  let output = "";
  let index = 0;

  while (index < source.length) {
    if (source[index] === "/" && source[index + 1] === "/") {
      while (index < source.length && source[index] !== "\n") {
        index += 1;
      }
      continue;
    }

    if (source[index] === "/" && source[index + 1] === "*") {
      index += 2;
      let depth = 1;

      while (index < source.length && depth > 0) {
        if (source[index] === "/" && source[index + 1] === "*") {
          depth += 1;
          index += 2;
        } else if (source[index] === "*" && source[index + 1] === "/") {
          depth -= 1;
          index += 2;
        } else {
          index += 1;
        }
      }
      continue;
    }

    output += source[index];
    index += 1;
  }

  return output;
}

export type ProgramInstruction = {
  name: string;
  discriminator: string;
  filePath: string;
};

export function parseInstructions(source: string, filePath: string): ProgramInstruction[] {
  const cleanSource = stripRustComments(source);
  const instructions: ProgramInstruction[] = [];
  const fnRegex = /\bpub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*(?:mut\s+)?(?:[A-Za-z_][A-Za-z0-9_]*)\s*:\s*Context\s*</g;

  let match;
  while ((match = fnRegex.exec(cleanSource)) !== null) {
    const fnName = match[1];
    instructions.push({
      name: fnName,
      discriminator: computeInstructionDiscriminator(fnName),
      filePath
    });
  }

  return instructions;
}

function computeInstructionDiscriminator(fnName: string): string {
  const hash = createHash("sha256").update(`global:${fnName}`).digest();
  return "0x" + hash.subarray(0, 8).toString("hex");
}
