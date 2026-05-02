# Manage

Add, update, and organize papers in the user's Zotero library.

## Adding papers

Choose the command based on what the user provides:

| User gives you | Command | Example |
|---------------|---------|---------|
| DOI | `zotron items add-by-doi` | `zotron items add-by-doi 10.1016/j.jfineco.2024.01.001` |
| URL (CNKI, journal site) | `zotron items add-by-url` | `zotron items add-by-url "https://..."` |
| ISBN | `zotron items add-by-isbn` | `zotron items add-by-isbn 978-7-...` |
| Local PDF file | `zotron items add-from-file` | `zotron items add-from-file /path/to/paper.pdf` |
| Manual entry | `zotron items create` | See below |

```bash
# By DOI (most reliable)
zotron items add-by-doi 10.1016/j.jfineco.2024.01.001

# With collection
zotron items add-by-doi 10.1016/j.jfineco.2024.01.001 --collection "数字经济"

# Import local PDF
zotron items add-from-file /path/to/paper.pdf --collection "数字经济"

# Manual creation
zotron items create --type journalArticle --field title="论文标题" --field date="2024" --field publicationTitle="经济研究"
```

## Updating metadata

```bash
zotron items update 12345 --field title="修正后的标题" --field date="2024-06"
```

## Collections (folders)

```bash
# Create
zotron collections create "数字经济文献"
zotron collections create "子文件夹" --parent "数字经济文献"

# Add papers to collection
zotron collections add-items "数字经济文献" 10 13 16

# Remove from collection (doesn't delete paper)
zotron collections remove-items "数字经济文献" 10

# Rename
zotron collections rename "typo-名称" "正确名称"

# Delete collection (items themselves are kept in library)
zotron collections delete "临时文件夹"
```

## Tags

```bash
# Add tags to a paper
zotron tags add 12345 --tag "核心" --tag "待读"

# Remove tags
zotron tags remove 12345 --tag "待读"

# Batch: tag multiple papers at once
zotron tags batch-update 10 13 16 --add "已读" --remove "待读"

# List all tags
zotron tags list

# Rename a tag library-wide
zotron tags rename "todo" "to-read"

# Delete a tag from all items
zotron tags delete "outdated-tag"
```

## Trash and delete

```bash
# Move to trash (reversible)
zotron items trash 12345

# Restore from trash
zotron items restore 12345

# Batch trash
zotron items batch-trash 12345 12346 12347

# View trashed items
zotron items list-trash

# Permanent delete (irreversible!)
zotron items delete 12345
```

## Related items

```bash
# View related items
zotron items related 12345

# Link two items as related
zotron items add-related 12345 --target 67890

# Unlink
zotron items remove-related 12345 --target 67890
```

## Duplicates

```bash
# Find duplicate papers
zotron items find-duplicates

# Merge duplicates (keeps first, merges others into it)
zotron items merge-duplicates 10 25
```

## Notes

```bash
# List notes on a paper
zotron notes list --parent 12345

# Create a note
zotron notes create --parent 12345 --content "重要发现：..." --tag research

# Update a note
zotron notes update <note-id> --content "修改后内容"

# Delete
zotron notes delete <note-id>

# Search notes
zotron notes search "量化分析" --limit 20
```

## After adding

Report to user: "已添加到 Zotero: {title} — {authors} ({year}) {journal}"
