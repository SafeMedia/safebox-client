export function getFileNameWithoutExtension(fileName: string): string {
    return fileName.replace(/\.[^/.]+$/, "");
}
