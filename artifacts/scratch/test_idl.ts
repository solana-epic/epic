import { analyzeAnchorIdl } from "/Users/aksh/Documents/Solana EPIC/packages/parser/src/project.ts";
import path from "node:path";

async function main() {
  const idlPath = path.resolve(process.argv[2]);
  console.log(`Analyzing IDL at: ${idlPath}`);
  const result = await analyzeAnchorIdl(idlPath);
  console.log(`Parsed ${result.accounts.length} accounts:`);
  for (const acc of result.accounts) {
    console.log(`- Account: ${acc.name} (${acc.byteSize} bytes)`);
    for (const f of acc.fields) {
      console.log(`  * ${f.name}: ${f.type} (${f.byteSize} bytes, dynamic=${f.dynamic})`);
    }
  }
}

main().catch(console.error);
