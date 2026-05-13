# Manage

Add, update, and organize papers in the user's Zotero library.

## Adding papers

Choose the command based on what the user provides:

| User gives you | Command | Example |
|---------------|---------|---------|
| DOI | `zotron items add --doi` | `zotron items add --doi 10.1016/j.jfineco.2024.01.001` |
| URL (journal site) | `zotron items add --from-url` | `zotron items add --from-url "https://..."` |
| ISBN | `zotron items add --isbn` | `zotron items add --isbn 978-7-...` |
| Local PDF file | `zotron items add --file` | `zotron items add --file /path/to/paper.pdf` |
| Manual entry | `zotron items create` | See below |

```bash
# By DOI (most reliable)
zotron items add --doi 10.1016/j.jfineco.2024.01.001

# With collection
zotron items add --doi 10.1016/j.jfineco.2024.01.001 --collection "数字经济"

# Import local PDF
zotron items add --file /path/to/paper.pdf --collection "数字经济"

# By URL
zotron items add --from-url "https://doi.org/10.1234/example" --collection "My Papers"

# By ISBN
zotron items add --isbn 978-7-111-55555-5

# Manual creation
zotron items create --type journalArticle --field title="论文标题" --field date="2024" --field publicationTitle="经济研究"
```

## Updating metadata

```bash
zotron items update YR5BUGHG --field title="修正后的标题" --field date="2024-06"
```

## Collections (folders)

```bash
# Create
zotron collections create "数字经济文献"
zotron collections create "子文件夹" --parent "数字经济文献"

# Add papers to collection
zotron collections add-items "数字经济文献" YR5BUGHG ABC12345 DEF67890

# Remove from collection (doesn't delete paper)
zotron collections remove-items "数字经济文献" YR5BUGHG

# Rename
zotron collections rename "typo-名称" "正确名称"

# Delete collection (items themselves are kept in library)
zotron collections delete "临时文件夹"
```

## Tags

```bash
# Add tags to a paper
zotron tags add YR5BUGHG --tag "核心" --tag "待读"

# Remove tags
zotron tags remove YR5BUGHG --tag "待读"

# Batch: tag multiple papers at once
zotron tags add YR5BUGHG ABC12345 DEF67890 --tag "已读"
zotron tags remove YR5BUGHG ABC12345 DEF67890 --tag "待读"

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
zotron items trash YR5BUGHG

# Batch trash
zotron items trash YR5BUGHG ABC12345 DEF67890

# Restore from trash
zotron items restore YR5BUGHG

# View trashed items
zotron items list --trash

# Permanent delete (irreversible!)
zotron items delete YR5BUGHG
```

## Related items

```bash
# View related items
zotron items related YR5BUGHG

# Link two items as related
zotron items add-related YR5BUGHG --target ABC12345

# Unlink
zotron items remove-related YR5BUGHG --target ABC12345
```

## Duplicates

```bash
# Find duplicate papers
zotron items find-duplicates

# Merge duplicates (keeps first, merges others into it)
zotron items merge-duplicates YR5BUGHG ABC12345
```

## Attachments

```bash
# Add a local file as attachment
zotron attachments add --parent YR5BUGHG /path/to/paper.pdf

# Add a remote URL as attachment
zotron attachments add --from-url https://example.com/paper.pdf --parent YR5BUGHG

# Auto-find and attach PDF from online sources
zotron attachments find-pdf --parent YR5BUGHG

# Delete an attachment
zotron attachments delete ATT_KEY
```

## Notes

```bash
# List notes on a paper
zotron notes list --parent YR5BUGHG

# Create a note
zotron notes create --parent YR5BUGHG --content "重要发现：..." --tag research

# Update a note
zotron notes update <note-key> --content "修改后内容"

# Delete
zotron notes delete <note-key>

# Search notes
zotron notes search "量化分析" --limit 20
```

## Batch PDF fill

```bash
# Find and attach PDFs for all items missing one in a collection
zotron find-pdfs --collection "数字经济" --limit 20
```

## Push prepared items

```bash
# Push a JSON file of items to Zotero
zotron push items.json --collection "数字经济"

# Push from stdin
cat items.json | zotron push - --collection "数字经济"

# Dry run — show what would be sent
zotron push items.json --dry-run
```

## After adding

Report to user: "已添加到 Zotero: {title} — {authors} ({year}) {journal}"
