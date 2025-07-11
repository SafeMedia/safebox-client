import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import RootLayout from "./components/root-layout";
import AppRoutes from "./routes";
import { ToastContainer } from "react-toastify";
import "@/styles/toastify.css";

import { StorageProvider } from "./providers/storage-provider";

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <BrowserRouter>
            <StorageProvider>
                <RootLayout>
                    <main>
                        <AppRoutes />
                    </main>
                    <ToastContainer
                        position="bottom-right"
                        autoClose={3000}
                        hideProgressBar={false}
                        newestOnTop={true}
                        closeOnClick
                        rtl={false}
                        pauseOnFocusLoss
                        draggable
                        pauseOnHover
                        theme="light"
                    />
                </RootLayout>
            </StorageProvider>
        </BrowserRouter>
    </StrictMode>
);
