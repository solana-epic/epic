type SizedType = {
  byteSize: number | null;
  dynamic: boolean;
  note?: string;
};

const PRIMITIVE_SIZES = new Map<string, number>([
  ["bool", 1],
  ["u8", 1],
  ["i8", 1],
  ["u16", 2],
  ["i16", 2],
  ["u32", 4],
  ["i32", 4],
  ["f32", 4],
  ["u64", 8],
  ["i64", 8],
  ["f64", 8],
  ["u128", 16],
  ["i128", 16],
  ["Pubkey", 32],
  ["publicKey", 32]
]);

export function sizeOfRustType(type: string): SizedType {
  const normalizedType = unwrapWhitespace(type);
  const primitiveSize = PRIMITIVE_SIZES.get(normalizedType);

  if (primitiveSize !== undefined) {
    return { byteSize: primitiveSize, dynamic: false };
  }

  const array = parseArrayType(normalizedType);

  if (array) {
    const inner = sizeOfRustType(array.innerType);
    return inner.byteSize === null
      ? { byteSize: null, dynamic: inner.dynamic, note: `unsupported array element type: ${array.innerType}` }
      : {
          byteSize: inner.byteSize * array.length,
          dynamic: inner.dynamic,
          ...(inner.note ? { note: inner.note } : {})
        };
  }

  const optionInner = parseGenericType(normalizedType, "Option");

  if (optionInner) {
    const inner = sizeOfRustType(optionInner);
    return inner.byteSize === null
      ? { byteSize: null, dynamic: inner.dynamic, note: `unsupported Option inner type: ${optionInner}` }
      : {
          byteSize: 1 + inner.byteSize,
          dynamic: inner.dynamic,
          ...(inner.note ? { note: inner.note } : {})
        };
  }

  const vecInner = parseGenericType(normalizedType, "Vec");

  if (vecInner) {
    return { byteSize: 4, dynamic: true, note: `Vec<${vecInner}> is dynamically sized; counted 4-byte length prefix only` };
  }

  if (normalizedType === "String") {
    return { byteSize: 4, dynamic: true, note: "String is dynamically sized; counted 4-byte length prefix only" };
  }

  const hashMapInner = parseGenericType(normalizedType, "HashMap");

  if (hashMapInner) {
    return { byteSize: 4, dynamic: true, note: `HashMap<${hashMapInner}> is dynamically sized; counted 4-byte length prefix only` };
  }

  const hashSetInner = parseGenericType(normalizedType, "HashSet");

  if (hashSetInner) {
    return { byteSize: 4, dynamic: true, note: `HashSet<${hashSetInner}> is dynamically sized; counted 4-byte length prefix only` };
  }

  const bTreeMapInner = parseGenericType(normalizedType, "BTreeMap");

  if (bTreeMapInner) {
    return { byteSize: 4, dynamic: true, note: `BTreeMap<${bTreeMapInner}> is dynamically sized; counted 4-byte length prefix only` };
  }

  const bTreeSetInner = parseGenericType(normalizedType, "BTreeSet");

  if (bTreeSetInner) {
    return { byteSize: 4, dynamic: true, note: `BTreeSet<${bTreeSetInner}> is dynamically sized; counted 4-byte length prefix only` };
  }

  return { byteSize: null, dynamic: false, note: `unsupported fixed size type: ${normalizedType}` };
}

function parseArrayType(type: string): { innerType: string; length: number } | null {
  const match = /^\[(.+);\s*(\d+)\]$/.exec(type);

  if (!match) {
    return null;
  }

  return {
    innerType: unwrapWhitespace(match[1]),
    length: Number(match[2])
  };
}

function parseGenericType(type: string, genericName: string): string | null {
  const prefix = `${genericName}<`;

  if (!type.startsWith(prefix) || !type.endsWith(">")) {
    return null;
  }

  return unwrapWhitespace(type.slice(prefix.length, -1));
}

function unwrapWhitespace(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}
