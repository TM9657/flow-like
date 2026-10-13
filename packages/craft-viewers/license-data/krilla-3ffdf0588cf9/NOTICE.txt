# Acknowledgements

Some parts of this code builds upon code copied from other crates. In particular:

## `resvg`

The following code snippets have been taken/adapted from [`resvg`](https://github.com/RazrFalcon/resvg),
available under the [MPL license](https://github.com/RazrFalcon/resvg/blob/master/LICENSE.txt):

- The contents of the `content_draw_path` method.
- The whole resvg test suite in `assets/svgs`.

## `typst`

The following code snippets have been taken/adapted from [`typst`](https://github.com/typst/typst),
available under the [Apache 2 license](https://github.com/typst/typst/blob/main/LICENSE):

- The `GroupByKey` struct.
- The `SliceExt` trait.
- The `Prehashed` struct.
- The implementation of `SipHashable`.
- The implementation of writing CID-keyed fonts.
- The implementation of writing PDF metadata.

## `svg2pdf`

The following code snippets have been taken/adapted from [`svg2pdf`](https://github.com/typst/svg2pdf),
available under the [Apache 2 license](https://github.com/typst/svg2pdf/blob/main/LICENSE-APACHE):

- The whole implementation of the SVG conversion.

## `vello`

The following code snippets have been taken/adapted from [`vello`](https://github.com/linebender/vello),
available under the [Apache 2 license](https://github.com/linebender/vello/blob/main/LICENSE-APACHE):

- The logic for bitmap glyphs in `bitmap.rs`.
