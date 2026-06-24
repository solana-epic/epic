import json
import os
import shutil

packages = [
    "cli",
    "parser",
    "diff-engine",
    "github-action",
    "cli-darwin-arm64",
    "cli-darwin-x64",
    "cli-linux-x64",
    "cli-win32-x64"
]

root_dir = "/Users/aksh/Documents/Solana EPIC"
target_version = "0.1.0-beta.2"

metadata = {
    "license": "MIT",
    "homepage": "https://github.com/solana-epic/epic#readme",
    "repository": {
        "type": "git",
        "url": "git+https://github.com/solana-epic/epic.git"
    },
    "bugs": {
        "url": "https://github.com/solana-epic/epic/issues"
    },
    "keywords": ["solana", "anchor", "security", "static-analysis", "audit", "upgrade-safety", "rust"],
    "author": "Solana EPIC Team"
}

# Update package.json files
for pkg in packages:
    pkg_dir = os.path.join(root_dir, "packages", pkg)
    pkg_json_path = os.path.join(pkg_dir, "package.json")
    
    if not os.path.exists(pkg_json_path):
        print(f"Skipping {pkg} (no package.json)")
        continue
        
    with open(pkg_json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
        
    # Bump version
    data["version"] = target_version
    
    # Apply metadata
    for k, v in metadata.items():
        data[k] = v
        
    # Update internal dependencies to beta.2
    for dep_key in ["dependencies", "optionalDependencies", "peerDependencies", "devDependencies"]:
        if dep_key in data:
            for dep in list(data[dep_key].keys()):
                if dep.startswith("@solana-epic/"):
                    data[dep_key][dep] = f"^{target_version}"
                    
    with open(pkg_json_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
        f.write("\n")
    print(f"Updated package.json: {pkg}")

# Copy README to main publishable modules
src_readme = os.path.join(root_dir, "README.md")
for pkg in ["cli", "parser", "diff-engine", "github-action"]:
    dest = os.path.join(root_dir, "packages", pkg, "README.md")
    shutil.copy2(src_readme, dest)
    print(f"Copied README to packages/{pkg}")

# Bump version in root package.json as well
root_json_path = os.path.join(root_dir, "package.json")
with open(root_json_path, "r", encoding="utf-8") as f:
    root_data = json.load(f)
root_data["version"] = target_version
with open(root_json_path, "w", encoding="utf-8") as f:
    json.dump(root_data, f, indent=2)
    f.write("\n")
print("Updated root package.json version")
