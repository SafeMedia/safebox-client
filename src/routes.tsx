import { Routes, Route } from "react-router-dom";
import Dashboard from "./components/dashboard";
import Settings from "./pages/settings";
import StorageSettings from "./components/settings/storage-settings";
import NotificationSettings from "./components/settings/notification-settings";
import PreferenceSettings from "./components/settings/preference-settings";

const AppRoutes = () => {
    return (
        <Routes>
            <Route path="/" element={<Dashboard />} />

            <Route path="/settings/*" element={<Settings />}>
                <Route path="storage" element={<StorageSettings />} />
                <Route
                    path="notifications"
                    element={<NotificationSettings />}
                />
                <Route path="preference" element={<PreferenceSettings />} />
            </Route>
        </Routes>
    );
};

export default AppRoutes;
