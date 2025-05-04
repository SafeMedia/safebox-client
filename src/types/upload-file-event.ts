export type UploadPayload = {
    name: string;
    mime_type: string;
    success: boolean;
    xorname?: string;
    error?: {
        title: string;
        description: string;
    };
};
