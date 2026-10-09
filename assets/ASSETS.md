# InkPage icon

Original paper, ink stroke and pen nib icon, generated with the built-in image_gen tool. The original transparent image is saved as inkpage-source.png. The exported 16, 20, 24, 32, 40, 48, 64, 128 and 256 pixel PNGs preserve transparency. InkPage.ico contains all nine sizes. The viewport embeds inkpage-256.png; build.rs embeds the ICO and Windows version information. No third-party editor logo was used.

To export again without changing the artwork: `python scripts/prepare_icon.py assets/inkpage-source.png` (Pillow required).

Final prompt used with the built-in tool, transparent_background=true:

> Use case: logo-brand
>
> Asset type: production Windows desktop application icon for 墨頁 InkPage, a compact text editor.
>
> Primary request: create one original polished app icon combining a cream paper page with folded top corner and a bold ink nib/ink stroke to suggest writing. A deep midnight-teal rounded-square tile supports the mark, with a restrained turquoise accent on the nib. Strong simple silhouette, large centered mark, designed to remain recognizable at 16 and 32 pixels. Modern restrained flat icon with very subtle dimensional paper shading, crisp edges, few large shapes.
>
> Composition: square image, icon tile occupies about 90% of image, transparent margin outside tile. Single icon only.
>
> Constraints: genuinely transparent pixels outside the rounded square, no background scenery, no text, no letters, no watermark, no existing editor or company logo, no thin decorative details, no mockup or multiple variants.
