## Inflate anki flashcards

Currently this tool just exists to take an Anki deck in .apkg form and convert all Chinese text in it to add Pleco links and tone tags.

For example, "你怎么样" gets converted to:
```html
<a href="plecoapi://x-callback-url/s?q=你怎么样" style="text-decoration:none">
    <span class="tone3">你</span>
    <span class="tone3">怎</span>
    <span class="tone5">么</span>
    <span class="tone4">样</span>
</a>
```

To use this:
1. Export your deck in .apkg form (remember to tick the boxes that asks if you want to include scheduling information and media)
2. Run this program on it - by default this will produce `out.apkg`.
3. Delete your original deck (don't worry, if I've messed up you can always re-import your deck that you exported).
4. Import `out.apkg`
5. If you want to take advantage of the tone tags, [add some CSS](https://docs.ankiweb.net/templates/styling.html) - for e.g. something like
```css
.tone1 {color: #00e304;}
.tone2 {color: #b35815;}
.tone3 {color: #f00f0f;}
.tone4 {color: #1767fe;}
.tone5 {color: #777777;}
```


### TODO
 - Add tone colours CSS automatically
 - Add options to perhaps not modify the front of the card