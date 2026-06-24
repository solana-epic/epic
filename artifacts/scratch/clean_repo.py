import os
import subprocess
import shutil

# Paths
repo_root = "/Users/aksh/Documents/Solana EPIC"
archive_dir = os.path.join(repo_root, "docs", "archive")
historical_dir = os.path.join(archive_dir, "historical")
release_dir = os.path.join(archive_dir, "release")

os.makedirs(historical_dir, exist_ok=True)
os.makedirs(release_dir, exist_ok=True)

# 14 Historical files to retain
historical_files = [
    "epic_sec_001_owner_validation_spec.md",
    "epic_sec_001_implementation_plan.md",
    "epic_sec_001_final_hostile_architecture_signoff.md",
    "EPIC_SEC_002_SPEC.md",
    "EPIC_SEC_002_DESIGN_REVIEW.md",
    "EPIC_SEC_003_SPEC.md",
    "EPIC_SEC_003_DESIGN_REVIEW.md",
    "EPIC_SEC_004_SPEC.md",
    "EPIC_SEC_004_DESIGN_REVIEW.md",
    "EPIC_SEC_005_SPEC.md",
    "EPIC_SEC_005_DESIGN_REVIEW.md",
    "epic_upgrade_safety_mvp_spec.md",
    "EPIC_PARSER_V3_FINAL_ARCHITECTURE.md",
    "EPIC_GUARDFACT_FINAL_ARCHITECTURE.md"
]

# Move EPIC_RELEASE_CANDIDATE_AUDIT.md if present
c_audit = "EPIC_RELEASE_CANDIDATE_AUDIT.md"
src_c_audit = os.path.join(archive_dir, c_audit)
dest_c_audit = os.path.join(release_dir, c_audit)
if os.path.exists(src_c_audit):
    subprocess.run(["git", "mv", src_c_audit, dest_c_audit], cwd=repo_root)
    print(f"Moved {c_audit} to release/")

moved_to_historical = []
deleted_files = []

# Process all files in docs/archive
for name in os.listdir(archive_dir):
    full_path = os.path.join(archive_dir, name)
    if os.path.isdir(full_path):
        continue
    
    # Check if this is a historical file to retain
    if name in historical_files:
        dest_path = os.path.join(historical_dir, name)
        subprocess.run(["git", "mv", full_path, dest_path], cwd=repo_root)
        moved_to_historical.append(name)
        print(f"Moved to historical: {name}")
    else:
        # We delete obsolete, duplicate, or planning files
        # Check if it's tracked in git
        res = subprocess.run(["git", "ls-files", full_path], capture_output=True, text=True, cwd=repo_root)
        if res.stdout.strip():
            subprocess.run(["git", "rm", "-f", full_path], cwd=repo_root)
        else:
            os.remove(full_path)
        deleted_files.append(name)
        print(f"Deleted: {name}")

print("\n--- Summary ---")
print(f"Moved to historical ({len(moved_to_historical)}):", moved_to_historical)
print(f"Deleted ({len(deleted_files)}):", deleted_files)
