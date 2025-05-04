export const generateLocation = (
    fileName: string,
    extension: string,
    downloadFolder: string
): string => {
    if (!fileName || !extension || !downloadFolder) {
        throw new Error(
            "All parameters (fileName, extension, downloadFolder) are required."
        );
    }

    // ensure the folder path is formatted correctly
    const safeDownloadFolder = downloadFolder.replace(/\/+$/, ""); // remove trailing slashes

    // construct the raw file path (no encoding here)
    const filePath = `${safeDownloadFolder}/${fileName.trim()}.${extension.trim()}`;

    return filePath;
};

export function extractFromFullPath(fullPath: string): {
    fileName: string;
    extension: string;
    folderPath: string;
} {
    const pathParts = fullPath.split(/[/\\]/);
    const fileNameWithExt = pathParts.pop() || "";
    const folderPath = pathParts.join("/");

    const lastDotIndex = fileNameWithExt.lastIndexOf(".");
    const fileName =
        lastDotIndex !== -1
            ? fileNameWithExt.slice(0, lastDotIndex)
            : fileNameWithExt;
    const extension =
        lastDotIndex !== -1 ? fileNameWithExt.slice(lastDotIndex + 1) : "";

    return { fileName, extension, folderPath };
}
