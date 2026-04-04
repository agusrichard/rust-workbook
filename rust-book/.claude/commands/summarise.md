---
description: Summarise a web page into Markdown format
argument-hint: <url>
allowed-tools: [WebFetch, Write, Read, Glob]
---

Please access and read the content of the following web page: $ARGUMENTS

Provide a comprehensive summary of the page in Markdown format. The summary should include:
- A clear title
- A high-level overview
- Key points and main arguments
- Important concepts
- Any significant conclusions or data points
- Code snippets

Please create a file and put the summary there. Here's the rule of the naming and where to put it:
- The file should be named as the title of the web page. For example, summary for url "https://rust-book.cs.brown.edu/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html" becomes "7.3-paths-for-referring-to-an-item-in-the-module-tree.md".
- The file should be put inside the folder of the chapter, for example the folder for chapter 7 is "7-managing-growing-projects-with-packages-crates-and-modules"
