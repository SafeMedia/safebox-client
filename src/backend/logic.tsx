import {
    connectInner,
    signIn as autonomiSignIn,
    register,
    clientAddress,
    isConnected,
    listAccounts,
    sessionSet,
    sessionRead,
    download as autonomiDownload,
} from "@/backend/autonomi";
import {
    AccountUser,
    RegisterAccountUser,
    SimpleAccountUser,
    RecoverAccountUser,
} from "@/types/account-user";
import {
    getDownloadFolder,
    getSelectedNetwork,
    getTestnetPeerAddress,
} from "@/backend/backend-store";
import Networks from "@/enums/networks";
import { isEthereumAddress } from "@/lib/utils/address";
import { AutonomiFile } from "@/types/autonomi-file";

// =======
// this file contains higher-level backend code with some application logic, and can use frontend types.
// =======

const USER_SESSION_KEY = "user";

export async function connect(override?: {
    network: Networks;
    peer?: string;
}): Promise<boolean> {
    console.log("connecting...");
    try {
        let peer = undefined;
        let network = undefined;

        // this is used if connecting from the disconnected-panel component
        if (override && override.network) {
            network = override.network;
            if (network == Networks.TESTNET) {
                peer = override.peer;
            }
        } else {
            network = await getSelectedNetwork();
            if (network == Networks.TESTNET) {
                peer = await getTestnetPeerAddress();
            }
        }

        if (network == Networks.TESTNET && !peer) {
            console.error("Peer not supplied for TESTNET.");
            return false;
        }

        const success = await connectInner(peer ?? undefined);
        if (success) {
            console.log("connected.");
            return true;
        }
    } catch (e) {
        console.error("connect: ", e);
    }
    return false;
}

export async function registerUser(
    newUser: RegisterAccountUser | RecoverAccountUser
): Promise<AccountUser | null> {
    console.log(`Attempting to create a new user: ${newUser.username}`);

    try {
        // register and connect the user
        let success = false;
        if ((newUser as RecoverAccountUser).privateKey !== undefined) {
            success = await register(
                newUser.username,
                newUser.password,
                (newUser as RecoverAccountUser).privateKey
            );
        } else {
            success = await register(newUser.username, newUser.password);
        }
        if (!success) {
            return null;
        }

        // retrieve the client address
        const address = await clientAddress();

        // check if address is null and handle it if needed
        if (address === null) {
            console.error(
                `Failed to retrieve address for user: ${newUser.username}`
            );
            return null;
        }

        // check if address is not a valid address
        if (!isEthereumAddress(address)) {
            console.error(
                `Failed to retrieve a valid address for user: ${newUser.username}`
            );
            return null;
        }

        const registeredUser = { ...newUser, address };
        registeredUser.password = ""; // we cannot save passwords

        console.log(`User ${newUser.username} created successfully.`);
        return registeredUser;
    } catch (error) {
        console.error(
            `Error during user registration for ${newUser.username}:`,
            error
        );
        return null;
    }
}

export async function signIn(
    username: string,
    password: string
): Promise<boolean> {
    let success = await autonomiSignIn(username, password);
    if (success) {
        const address = await clientAddress();
        if (address) {
            await sessionSet(
                USER_SESSION_KEY,
                JSON.stringify({
                    username: username,
                    address: address,
                })
            );
        } else {
            success = false;
        }
    }
    return success;
}

export async function signOut(): Promise<void> {
    try {
        await sessionSet(USER_SESSION_KEY, null);
    } catch (e) {
        console.error("signOut: ", e);
    }
}

export async function saveUser(user: AccountUser) {
    console.log("saving user: ", user);
    await sessionSet(USER_SESSION_KEY, JSON.stringify(user));
}

// returns user account object if account is connected, null if not.
export async function getConnectedUserAccount(): Promise<AccountUser | null> {
    try {
        let connected = await isConnected();

        if (connected) {
            console.log("getting user...");

            const user = await sessionRead(USER_SESSION_KEY);
            console.log("user: ", user);
            if (user) {
                return JSON.parse(user) as AccountUser;
            }

            console.log("User not found.");
        }
    } catch (e) {
        console.error("Unexpected error in getConnectedUserAccount: ", e);
    }

    return null;
}

// return all registered accounts with addresses, sorted from most recently used.
export async function registeredAccounts(): Promise<SimpleAccountUser[]> {
    // Fetch the accounts as an array of [username, address] tuples
    const accounts = await listAccounts();

    // if the accounts are not null, map the tuples to SimpleAccountUser objects
    // assuming accounts is an array of tuples [username, address]
    if (accounts) {
        // Reverse the accounts array
        accounts.reverse();

        // filter out invalid Ethereum addresses and then map the results
        return accounts
            .filter(([_username, address]) => isEthereumAddress(address)) // Filter valid addresses
            .map(([username, address]) => ({
                username,
                address,
            }));
    } else {
        return [];
    }
}

export async function download(
    xorname: string,
    fileName?: string,
    destinationDir?: string
): Promise<AutonomiFile | null> {
    console.log(
        `Starting download: ${xorname} => ${destinationDir || "default folder"}`
    );

    try {
        // get target directory (use provided dir, otherwise fetch default)
        const targetDir = destinationDir || (await getDownloadFolder());

        if (!targetDir) {
            console.error("No valid download directory found.");
            return null;
        }

        if (typeof targetDir !== "string") {
            console.error(
                "Invalid download directory format (should be string):",
                targetDir
            );
            return null;
        }

        console.log(`Downloading to: ${targetDir}`);

        // start the download
        const response = (await autonomiDownload(
            xorname,
            targetDir,
            fileName
        )) as AutonomiFile;

        console.log("download response:", response);

        // ensure response is correctly structured
        if (!response || typeof response !== "object") {
            console.error("Invalid response received from download function.");
            return null;
        }

        console.log("download file:", response);

        return response;
    } catch (error) {
        console.error("Download error:", error);
        return null;
    }
}
