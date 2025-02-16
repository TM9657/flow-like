import path from "path"
import fs from "fs"
import sharp, {

} from "sharp"

const extensions = new Set([
    ".png",
    ".jpg",
    ".jpeg"
])

async function convertToWebp(imgPath: string) {
    const newPath = path.join(path.dirname(imgPath), path.basename(imgPath, path.extname(imgPath)) + ".webp")
    const img = sharp(imgPath)
    await img.webp().toFile(newPath)
    return newPath
}

async function recursiveTransformImages(dir: string){
    const files = fs.readdirSync(dir)
    for(const file of files){
        const filePath = path.join(dir, file)
        if(fs.lstatSync(filePath).isDirectory()){
            await recursiveTransformImages(filePath)
        } else {
            if(extensions.has(path.extname(filePath))){
                await convertToWebp(filePath)
            }
        }
    }
}

recursiveTransformImages("apps/desktop/public")