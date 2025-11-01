![version](https://img.shields.io/badge/version-20%2B-E23089)
![platform](https://img.shields.io/static/v1?label=platform&message=mac-intel%20|%20mac-arm%20|%20win-64&color=blue)
[![license](https://img.shields.io/github/license/miyako/mermaid)](LICENSE)
![downloads](https://img.shields.io/github/downloads/miyako/mermaid/total)

# mermaid
Tool to convert mermaid markdown to SVG (namespace: `mermaid`)

## usage

There are `3` ways to use `mermaid`; atomic, asynchronous, server.

### atomic

```4d
var $mermaid : cs.mermaid.mermaid
$mermaid:=cs.mermaid.mermaid.new()
var $tasks : Collection
$tasks:=[]
var $file : Variant

$file:="graph TD\n    A[Start] --> B{Is it working?}\n    B -- Yes --> C[Great!]\n    B -- No --> D[Check the code]\n    D --> B\n    C --> E[End]"
$tasks.push({file: $file; data: Folder(fk desktop folder).file("2.svg")})

$results:=$mermaid.render($tasks)
```

### asynchronous

```4d
var $mermaid : cs.mermaid.mermaid
$mermaid:=cs.mermaid.mermaid.new()
var $tasks : Collection
$tasks:=[]
var $file : Variant
	
$file:="graph TD\n    A[Start] --> B{Is it working?}\n    B -- Yes --> C[Great!]\n    B -- No --> D[Check the code]\n    D --> B\n    C --> E[End]"
$tasks.push({file: $file; data: Folder(fk desktop folder).file("2.svg")})

$mermaid.render($tasks; Formula(onResponse))
```

the formula should have the following signature:

```4d
#DECLARE($worker : 4D.SystemWorker; $params : Object)

var $text : Text
$text:=$worker.response
```

> [!TIP]
> whatever value you pass in `data` is returned in `context`.

### server

```4d
var $mermaid : cs.mermaid.server
$mermaid:=cs.mermaid.server.new()
$mermaid.start(port: 8282)
```

`POST` a mermaid markdown to `/render`. If successful, `image/svg+xml` is returned.
