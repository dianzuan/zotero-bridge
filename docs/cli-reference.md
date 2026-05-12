# Zotron CLI 完整命令参考

生成时间: 2026-05-11 19:40

## zotron ping（这个没问题）
```
Check that Zotero is running with the Zotron XPI enabled

Usage: zotron ping [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron rpc（这个还需要暴露吗话说）
```
Generic RPC escape hatch

Usage: zotron rpc [OPTIONS] <METHOD> [PARAMS_JSON]

Arguments:
  <METHOD>       
  [PARAMS_JSON]  [default: {}]

Options:
      --url <URL>              [default: http://127.0.0.1:23119/zotron/rpc]
      --paginate               
      --page-size <PAGE_SIZE>  [default: 100]
  -h, --help                   Print help
```

## zotron push（这个没问题）
```
Push prepared Zotero JSON (from file or stdin) to Zotero

Usage: zotron push [OPTIONS] <JSON_FILE>

Arguments:
  <JSON_FILE>  Path to a JSON file, or "-" to read from stdin

Options:
      --pdf <PDF>                    Optional PDF attachment path
      --collection <COLLECTION>      Collection name (fuzzy) or key
      --on-duplicate <ON_DUPLICATE>  Duplicate handling: skip | update | create [default: skip]
      --url <URL>                    [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run                      Parse input + resolve collection only; do not push to Zotero
  -h, --help                         Print help
```

## zotron find-pdfs（这个的作用是？）
```
Batch fill missing PDFs in a collection via Zotero's resolver chain

Usage: zotron find-pdfs [OPTIONS] --collection <COLLECTION>

Options:
      --collection <COLLECTION>  
      --limit <LIMIT>            [default: 0]
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron system version
```
Show XPI version and exposed method metadata

Usage: zotron system version [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron system libraries
```
List all libraries (user + groups)

Usage: zotron system libraries [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron system library-stats
```
Get statistics for the current (or specified) library

Usage: zotron system library-stats [OPTIONS]

Options:
      --library <LIBRARY>  
      --url <URL>          [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help               Print help
```

## zotron system item-types
```
List all available Zotero item types

Usage: zotron system item-types [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron system item-fields
```
List all fields for a given item type

Usage: zotron system item-fields [OPTIONS] --type <ITEM_TYPE>

Options:
      --type <ITEM_TYPE>  
      --url <URL>         [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help              Print help
```

## zotron system creator-types
```
List creator types for a given item type

Usage: zotron system creator-types [OPTIONS] --type <ITEM_TYPE>

Options:
      --type <ITEM_TYPE>  
      --url <URL>         [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help              Print help
```

## zotron system current-collection
```
Get the currently selected Zotero collection (or null)

Usage: zotron system current-collection [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron system list-methods
```
List all RPC methods exposed by the XPI

Usage: zotron system list-methods [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron system describe
```
Describe one or all RPC methods (schema / signatures)

Usage: zotron system describe [OPTIONS] [METHOD]

Arguments:
  [METHOD]  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron search
```
Search items by text, tag, identifier, or structured conditions

Usage: zotron search [OPTIONS] [QUERY] [COMMAND]

Commands:
  saved-searches  List all saved searches in the library
  create-saved    Create a saved search with one or more conditions
  delete-saved    Delete a saved search by key

Arguments:
  [QUERY]  Search query (title/creator/year by default; PDF content with --fulltext)

Options:
      --fulltext                 Search inside PDF full-text content instead of metadata
      --author <AUTHOR>          Filter by author/creator name (contains match)
      --after <AFTER>            Filter by date after (YYYY or YYYY-MM-DD)
      --before <BEFORE>          Filter by date before (YYYY or YYYY-MM-DD)
      --journal <JOURNAL>        Filter by journal/publication title (contains match)
      --tag <TAG>                Filter by tag (exact match)
      --doi <DOI>                Find by DOI
      --isbn <ISBN>              Find by ISBN
      --issn <ISSN>              Find by ISSN
      --collection <COLLECTION>  Limit results to a collection name or key
      --limit <LIMIT>            [default: 50]
      --offset <OFFSET>          [default: 0]
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron search saved-searches
```
List all saved searches in the library

Usage: zotron search saved-searches [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron search create-saved
```
Create a saved search with one or more conditions

Usage: zotron search create-saved [OPTIONS] --condition <CONDITION> <NAME>

Arguments:
  <NAME>  

Options:
      --condition <CONDITION>  
      --dry-run                
      --url <URL>              [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                   Print help
```

## zotron search delete-saved
```
Delete a saved search by key

Usage: zotron search delete-saved [OPTIONS] <SEARCH_KEY>

Arguments:
  <SEARCH_KEY>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items add-by-doi
```
Add a paper by DOI using Zotero's search translators

Usage: zotron items add-by-doi [OPTIONS] <DOI>

Arguments:
  <DOI>  

Options:
      --collection <COLLECTION>  
      --dry-run                  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron items add-by-isbn
```
Add a book by ISBN

Usage: zotron items add-by-isbn [OPTIONS] <ISBN>

Arguments:
  <ISBN>  

Options:
      --collection <COLLECTION>  
      --dry-run                  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron items add-by-url
```
Add a web resource via Zotero's web translator

Usage: zotron items add-by-url [OPTIONS] <PAGE_URL>

Arguments:
  <PAGE_URL>  

Options:
      --collection <COLLECTION>  
      --dry-run                  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron items add-from-file
```
Add an item from a local file

Usage: zotron items add-from-file [OPTIONS] <PATH>

Arguments:
  <PATH>  

Options:
      --collection <COLLECTION>  
      --dry-run                  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron items create
```
Create a new item of the given type

Usage: zotron items create [OPTIONS] --type <ITEM_TYPE>

Options:
      --type <ITEM_TYPE>  
      --field <FIELDS>    
      --dry-run           
      --url <URL>         [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help              Print help
```

## zotron items update
```
Update fields on an existing item

Usage: zotron items update [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --field <FIELDS>  
      --dry-run         
      --url <URL>       [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help            Print help
```

## zotron items delete
```
Permanently delete an item

Usage: zotron items delete [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items trash
```
Move item to trash

Usage: zotron items trash [OPTIONS] <ITEM>

Arguments:
  <ITEM>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items restore
```
Restore a trashed item

Usage: zotron items restore [OPTIONS] <ITEM>

Arguments:
  <ITEM>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items batch-trash
```
Move multiple items to trash in one call

Usage: zotron items batch-trash [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items merge-duplicates
```
Merge a group of duplicate items

Usage: zotron items merge-duplicates [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items add-related
```
Add a related-item link between two items

Usage: zotron items add-related [OPTIONS] --target <TARGET> <KEY>

Arguments:
  <KEY>  

Options:
      --target <TARGET>  
      --dry-run          
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron items remove-related
```
Remove a related-item link between two items

Usage: zotron items remove-related [OPTIONS] --target <TARGET> <KEY>

Arguments:
  <KEY>  

Options:
      --target <TARGET>  
      --dry-run          
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron items get
```
Print the full serialization of an item by key

Usage: zotron items get [OPTIONS] <ITEM>

Arguments:
  <ITEM>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items list
```
List items in the library with optional sorting and pagination

Usage: zotron items list [OPTIONS]

Options:
      --limit <LIMIT>          [default: 50]
      --offset <OFFSET>        [default: 0]
      --sort <SORT>            
      --direction <DIRECTION>  [default: asc]
      --url <URL>              [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                   Print help
```

## zotron items find-duplicates
```
Run Zotero's duplicate scan and print groups

Usage: zotron items find-duplicates [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items list-trash
```
List items currently in the trash

Usage: zotron items list-trash [OPTIONS]

Options:
      --limit <LIMIT>    [default: 50]
      --offset <OFFSET>  [default: 0]
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron items recent
```
List recently added or modified items

Usage: zotron items recent [OPTIONS]

Options:
      --limit <LIMIT>       [default: 20]
      --offset <OFFSET>     [default: 0]
      --type <RECENT_TYPE>  [default: added]
      --url <URL>           [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                Print help
```

## zotron items fulltext
```
Retrieve the full-text content of an item's attachment

Usage: zotron items fulltext [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items related
```
List items related to the given item

Usage: zotron items related [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron items citation-key
```
Get the citation key for an item

Usage: zotron items citation-key [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron collections list
```
List all collections in the user library (flat)

Usage: zotron collections list [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron collections tree
```
Print the collection hierarchy as a tree

Usage: zotron collections tree [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron collections get
```
Get a single collection's metadata

Usage: zotron collections get [OPTIONS] <NAME_OR_ID>

Arguments:
  <NAME_OR_ID>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron collections get-items
```
List all items in a collection

Usage: zotron collections get-items [OPTIONS] <NAME_OR_ID>

Arguments:
  <NAME_OR_ID>  

Options:
      --limit <LIMIT>    
      --offset <OFFSET>  [default: 0]
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron collections stats
```
Show item/attachment/note/subcollection counts for a collection

Usage: zotron collections stats [OPTIONS] <NAME_OR_ID>

Arguments:
  <NAME_OR_ID>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron collections rename
```
Rename a collection

Usage: zotron collections rename [OPTIONS] <OLD_NAME> <NEW_NAME>

Arguments:
  <OLD_NAME>  
  <NEW_NAME>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run    
  -h, --help       Print help
```

## zotron collections create
```
Create a collection, optionally nested under a parent

Usage: zotron collections create [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --parent <PARENT>  
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run          
  -h, --help             Print help
```

## zotron collections delete
```
Delete a collection

Usage: zotron collections delete [OPTIONS] <NAME_OR_ID>

Arguments:
  <NAME_OR_ID>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run    
  -h, --help       Print help
```

## zotron collections add-items
```
Add existing items to a collection

Usage: zotron collections add-items [OPTIONS] <COLLECTION> [ITEM_KEYS]...

Arguments:
  <COLLECTION>    
  [ITEM_KEYS]...  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run    
  -h, --help       Print help
```

## zotron collections remove-items
```
Remove items from a collection

Usage: zotron collections remove-items [OPTIONS] <COLLECTION> [ITEM_KEYS]...

Arguments:
  <COLLECTION>    
  [ITEM_KEYS]...  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run    
  -h, --help       Print help
```

## zotron notes list
```
List notes attached to a parent item

Usage: zotron notes list [OPTIONS] --parent <PARENT>

Options:
      --parent <PARENT>  
      --limit <LIMIT>    [default: 50]
      --offset <OFFSET>  [default: 0]
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron notes get
```
Get a single note by key

Usage: zotron notes get [OPTIONS] <NOTE_KEY>

Arguments:
  <NOTE_KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron notes create
```
Create a note attached to a parent item

Usage: zotron notes create [OPTIONS] --parent <PARENT> --content <CONTENT>

Options:
      --parent <PARENT>    
      --content <CONTENT>  
      --tag <TAGS>         
      --dry-run            
      --url <URL>          [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help               Print help
```

## zotron notes update
```
Update the content of an existing note

Usage: zotron notes update [OPTIONS] --content <CONTENT> <NOTE_KEY>

Arguments:
  <NOTE_KEY>  

Options:
      --content <CONTENT>  
      --dry-run            
      --url <URL>          [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help               Print help
```

## zotron notes delete
```
Delete a note by key

Usage: zotron notes delete [OPTIONS] <NOTE_KEY>

Arguments:
  <NOTE_KEY>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron notes search
```
Search notes by text content

Usage: zotron notes search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  

Options:
      --limit <LIMIT>  [default: 50]
      --url <URL>      [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help           Print help
```

## zotron attachments list
```
List attachments belonging to a parent item

Usage: zotron attachments list [OPTIONS] --parent <PARENT>

Options:
      --parent <PARENT>  
      --limit <LIMIT>    [default: 50]
      --offset <OFFSET>  [default: 0]
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron attachments get
```
Get a single attachment by key

Usage: zotron attachments get [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron attachments fulltext
```
Get full-text content of an attachment

Usage: zotron attachments fulltext [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron attachments path
```
Get the local filesystem path of an attachment

Usage: zotron attachments path [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron attachments add
```
Attach a local file to an item

Usage: zotron attachments add [OPTIONS] --parent <PARENT> --path <PATH>

Options:
      --parent <PARENT>  
      --path <PATH>      
      --title <TITLE>    
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run          
  -h, --help             Print help
```

## zotron attachments add-by-url
```
Attach a remote file (by URL) to an item

Usage: zotron attachments add-by-url [OPTIONS] --parent <PARENT> --source-url <SOURCE_URL>

Options:
      --parent <PARENT>          
      --source-url <SOURCE_URL>  
      --title <TITLE>            
      --endpoint <ENDPOINT>      [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run                  
  -h, --help                     Print help
```

## zotron attachments delete
```
Delete an attachment

Usage: zotron attachments delete [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run    
  -h, --help       Print help
```

## zotron attachments find-pdf
```
Trigger Zotero's Find Available PDF for a parent item

Usage: zotron attachments find-pdf [OPTIONS] --parent <PARENT>

Options:
      --parent <PARENT>  
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron tags list
```
List all tags in the library (flat)

Usage: zotron tags list [OPTIONS]

Options:
      --limit <LIMIT>  [default: 200]
      --url <URL>      [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help           Print help
```

## zotron tags rename
```
Rename a tag across all items

Usage: zotron tags rename [OPTIONS] <OLD> <NEW>

Arguments:
  <OLD>  
  <NEW>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron tags delete
```
Delete a tag library-wide

Usage: zotron tags delete [OPTIONS] <TAG>

Arguments:
  <TAG>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron tags add
```
Add one or more tags to an item

Usage: zotron tags add [OPTIONS] --tag <TAGS> <KEY>

Arguments:
  <KEY>  

Options:
      --tag <TAGS>  
      --dry-run     
      --url <URL>   [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help        Print help
```

## zotron tags remove
```
Remove one or more tags from an item

Usage: zotron tags remove [OPTIONS] --tag <TAGS> <KEY>

Arguments:
  <KEY>  

Options:
      --tag <TAGS>  
      --dry-run     
      --url <URL>   [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help        Print help
```

## zotron tags batch-update
```
Batch add/remove tags across multiple items

Usage: zotron tags batch-update [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --add <ADD_TAGS>        
      --remove <REMOVE_TAGS>  
      --dry-run               
      --url <URL>             [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                  Print help
```

## zotron annotations list
```
List annotations on a PDF attachment

Usage: zotron annotations list [OPTIONS] --parent <PARENT>

Options:
      --parent <PARENT>  
      --url <URL>        [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help             Print help
```

## zotron annotations create
```
Create a new annotation on a PDF attachment

Usage: zotron annotations create [OPTIONS] --parent <PARENT> --type <ANNOTATION_TYPE>

Options:
      --parent <PARENT>          
      --type <ANNOTATION_TYPE>   
      --position <POSITION>      JSON annotation position, for example '{"pageIndex":0,"rects":[[10,20,30,40]]}'
      --sort-index <SORT_INDEX>  Zotero annotation sort index
      --text <TEXT>              
      --comment <COMMENT>        
      --color <COLOR>            [default: #ffd400]
      --dry-run                  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron annotations delete
```
Delete an annotation by key

Usage: zotron annotations delete [OPTIONS] <ANNOTATION_KEY>

Arguments:
  <ANNOTATION_KEY>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron settings get
```
Get a single Zotero preference value

Usage: zotron settings get [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron settings list
```
List all Zotero preferences as a key->value dict

Usage: zotron settings list [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron settings set
```
Set a single Zotero preference

Usage: zotron settings set [OPTIONS] <KEY> <VALUE>

Arguments:
  <KEY>    
  <VALUE>  

Options:
      --dry-run    
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron settings set-all
```
Bulk-set Zotero preferences from a JSON file

Usage: zotron settings set-all [OPTIONS] --file <FILE>

Options:
      --file <FILE>  
      --dry-run      
      --url <URL>    [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help         Print help
```

## zotron export bibtex
```
Print BibTeX for the given item keys

Usage: zotron export bibtex [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron export ris
```
Print RIS for the given item keys

Usage: zotron export ris [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron export csl-json
```
Print CSL-JSON for the given item keys

Usage: zotron export csl-json [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron export bibliography
```
Print a formatted bibliography

Usage: zotron export bibliography [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  

Options:
      --style <STYLE>  [default: http://www.zotero.org/styles/gb-t-7714-2015-numeric]
      --html           
      --url <URL>      [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help           Print help
```

## zotron ocr providers
```
Print supported OCR provider contracts

Usage: zotron ocr providers

Options:
  -h, --help  Print help
```

## zotron ocr provider-json
```
Execute an OCR provider request from JSON and emit normalized blocks

Usage: zotron ocr provider-json [OPTIONS] --provider <PROVIDER>

Options:
      --provider <PROVIDER>
          
      --input <INPUT>
          Path to an OcrRequestInput JSON file, or "-" to read stdin
      --file <FILE>
          Local PDF/image file to encode into an OcrRequestInput
      --item-key <ITEM_KEY>
          Zotero item key used when --file builds the OCR request
      --attachment-key <ATTACHMENT_KEY>
          Zotero attachment key used when --file builds the OCR request
      --mime-type <MIME_TYPE>
          MIME type used when --file builds the OCR request
      --endpoint <ENDPOINT>
          Override the provider endpoint, required for service-hosted PaddleOCR-VL
      --api-key-env <API_KEY_ENV>
          Environment variable containing the provider bearer token
  -h, --help
          Print help
```

## zotron ocr status
```
Show OCR statistics for a collection

Usage: zotron ocr status [OPTIONS] --collection <COLLECTION>

Options:
      --collection <COLLECTION>  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron ocr parse-pdf
```
Parse a Zotero PDF through MinerU and write hidden sidecar OCR/RAG artifacts

Usage: zotron ocr parse-pdf [OPTIONS] --parent <PARENT> --attachment <ATTACHMENT>

Options:
      --provider <PROVIDER>
          [default: mineru]
      --parent <PARENT>
          Parent Zotero item key
      --attachment <ATTACHMENT>
          Zotero PDF attachment key
      --source-url <SOURCE_URL>
          Public URL for MinerU cloud parsing. Use --result-dir/--result-zip for offline ingestion
      --result-dir <RESULT_DIR>
          Already-extracted MinerU result directory, used by tests/offline replay
      --result-zip <RESULT_ZIP>
          Already-downloaded MinerU result zip, used by tests/offline replay
      --provider-endpoint <PROVIDER_ENDPOINT>
          Override MinerU submit endpoint
      --api-key-env <API_KEY_ENV>
          Environment variable containing the MinerU bearer token [default: ZOTRON_MINERU_API_KEY]
      --poll-interval-seconds <POLL_INTERVAL_SECONDS>
          [default: 5]
      --timeout-seconds <TIMEOUT_SECONDS>
          [default: 900]
      --chunk-chars <CHUNK_CHARS>
          [default: 1200]
      --url <URL>
          [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help
          Print help
```

## zotron rag embedding-providers
```
Print supported embedding provider contracts

Usage: zotron rag embedding-providers

Options:
  -h, --help  Print help
```

## zotron rag embedding-json
```
Execute an embedding provider request from JSON and emit vectors

Usage: zotron rag embedding-json [OPTIONS] --provider <PROVIDER> --input <INPUT>

Options:
      --provider <PROVIDER>        
      --input <INPUT>              Path to an EmbeddingRequestInput JSON file, or "-" to read stdin
      --endpoint <ENDPOINT>        Override the embedding endpoint
      --model <MODEL>              Override the embedding model
      --input-type <INPUT_TYPE>    Override provider input type, for example document or query
      --api-key-env <API_KEY_ENV>  Environment variable containing the provider bearer token
  -h, --help                       Print help
```

## zotron rag status
```
Show index status for a collection

Usage: zotron rag status [OPTIONS] --collection <COLLECTION>

Options:
      --collection <COLLECTION>  
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                     Print help
```

## zotron rag hits
```
Emit academic-zh retrieval hits with item_key/title/text provenance

Usage: zotron rag hits [OPTIONS] <QUERY>

Arguments:
  <QUERY>  

Options:
      --collection <COLLECTION>                  
      --key <KEYS>                               Limit retrieval to one or more Zotero item keys
      --zotero                                   
      --top-spans-per-item <TOP_SPANS_PER_ITEM>  [default: 3]
      --include-fulltext-spans                   
      --limit <TOP_K>                            [default: 50]
      --output <OUTPUT>                          [default: json] [possible values: json, jsonl]
      --url <URL>                                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                                     Print help
```

