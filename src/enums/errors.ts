export enum ErrorKeys {
    PaymentRequired = "PaymentRequired",
    NotLoggedIn = "NotLoggedIn",
    UnknownError = "UnknownError",
}

export const Errors: Record<ErrorKeys, { title: string; description: string }> =
    {
        [ErrorKeys.PaymentRequired]: {
            title: "Payment Required",
            description:
                "You don't have enough funds to proceed with the upload.",
        },
        [ErrorKeys.NotLoggedIn]: {
            title: "Not Logged In",
            description: "You are not logged in.",
        },
        [ErrorKeys.UnknownError]: {
            title: "Unknown Error",
            description:
                "An unexpected error occurred. Please try again later.",
        },
    };
