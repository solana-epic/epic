import * as github from "@actions/github";
import * as core from "@actions/core";

export async function upsertPRComment(token: string, reportMarkdown: string): Promise<void> {
  const context = github.context;
  
  if (!context.payload.pull_request) {
    core.info("Not a pull request event. Skipping comment posting.");
    return;
  }

  const prNumber = context.payload.pull_request.number;
  const owner = context.repo.owner;
  const repo = context.repo.repo;

  const octokit = github.getOctokit(token);

  const commentHeader = "<!-- epic-upgrade-guard-comment -->";
  const bodyWithHeader = `${commentHeader}\n${reportMarkdown}`;

  core.info(`Searching for existing EPIC comments on PR #${prNumber}...`);

  const { data: comments } = await octokit.rest.issues.listComments({
    owner,
    repo,
    issue_number: prNumber,
    per_page: 100
  });

  const existingComment = comments.find((comment) => comment.body?.includes(commentHeader));

  if (existingComment) {
    core.info(`Found existing comment (ID: ${existingComment.id}). Updating it to avoid comment spam...`);
    await octokit.rest.issues.updateComment({
      owner,
      repo,
      comment_id: existingComment.id,
      body: bodyWithHeader
    });
  } else {
    core.info(`No existing comment found. Creating a new one...`);
    await octokit.rest.issues.createComment({
      owner,
      repo,
      issue_number: prNumber,
      body: bodyWithHeader
    });
  }
}

export async function checkIfConfigChanged(token: string): Promise<boolean> {
  const context = github.context;
  
  if (!context.payload.pull_request) {
    core.info("Not a pull request context. Skipping config change audit.");
    return false;
  }

  const prNumber = context.payload.pull_request.number;
  const owner = context.repo.owner;
  const repo = context.repo.repo;
  const octokit = github.getOctokit(token);

  try {
    core.info(`Auditing PR #${prNumber} files for modifications to epic.toml...`);
    const { data: files } = await octokit.rest.pulls.listFiles({
      owner,
      repo,
      pull_number: prNumber,
      per_page: 100
    });

    const isModified = files.some(file => file.filename.endsWith("epic.toml"));
    core.info(`Config modification audit: ${isModified ? "MODIFIED" : "UNMODIFIED"}`);
    return isModified;
  } catch (error) {
    core.warning(`Failed to list pull request files from GitHub API: ${error}`);
    return false;
  }
}

