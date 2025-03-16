import sys
import os
from pathlib import Path
from docling.datamodel.base_models import InputFormat
from docling.datamodel.pipeline_options import PdfPipelineOptions
from docling.document_converter import DocumentConverter, PdfFormatOption
from docling.utils.model_downloader import download_models
from docling_core.types.doc import ImageRefMode

IMAGE_RESOLUTION_SCALE = 2.0

def main():
    if len(sys.argv) < 4:
        print("Usage: python main.py <source> <target> <cache>")
        print("  <source>: Source URL or file path")
        print("  <target>: Target file path to save the markdown output")
        print("  <cache>: Directory path for storing model artifacts")
        return os._exit(0)

    source = sys.argv[1]
    target = sys.argv[2]
    cache = sys.argv[3]

    os.environ["HF_HOME"] = str(Path(cache).joinpath("huggingface"))

    source_path = Path(source)
    if not source_path.exists():
        print(f"Source file {source} does not exist")
        return os._exit(0)

    print(f"Converting {source} to markdown and saving to {target} - Cache: {cache}")

    artifacts_path = cache
    print(f"Downloading models to {artifacts_path}")
    download_models(output_dir=Path(artifacts_path), progress=True, force=False)

    # Convert the document
    pipeline_options = PdfPipelineOptions(artifacts_path=artifacts_path)
    pipeline_options.images_scale = IMAGE_RESOLUTION_SCALE
    pipeline_options.generate_picture_images = True

    converter = DocumentConverter(
        format_options={
            InputFormat.PDF: PdfFormatOption(pipeline_options=pipeline_options)
        }
    )
    result = converter.convert(source)

    # Save the markdown output to the target file
    with open(target, "w", encoding="utf-8") as f:
        f.write(result.document.export_to_markdown(image_mode=ImageRefMode.EMBEDDED))

    print(f"Markdown saved to {target}")
    return os._exit(0)

if __name__ == "__main__":
    main()