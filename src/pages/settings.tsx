import { useLocation, useNavigate } from "react-router-dom";
import { Outlet } from "react-router-dom";
import { useEffect, useState } from "react";
import TopLevelBreadcrumbs from "@/components/navigation/top-level-breadcrumbs";
import SecondaryLevelBreadcrumbs from "@/components/navigation/secondary-level-breadcrumbs";
import { useTranslation } from "react-i18next";

export default function Settings() {
    const { t } = useTranslation();
    const navigate = useNavigate();
    const location = useLocation();

    const secondaryNavigation = [
        { name: t("storage"), href: "/settings/storage", current: false },
        {
            name: t("notifications"),
            href: "/settings/notifications",
            current: false,
        },
        {
            name: t("preferences"),
            href: "/settings/preference",
            current: false,
        },
    ];

    // initialize pages with default settings page
    const [pages, setPages] = useState([
        { name: t("settings"), current: true },
    ]);

    // use effect to navigate to StatusSettings when the component mounts
    useEffect(() => {
        // determine the active secondary navigation item based on current location
        const currentNavItem = secondaryNavigation.find(
            (item) => item.href === location.pathname
        );
        // update pages state
        if (currentNavItem) {
            setPages([{ name: t("settings"), current: true }, currentNavItem]);
        } else {
            setPages([{ name: t("settings"), current: true }]);
        }

        // check if the current path is not set, then navigate to status by default
        if (location.pathname === "/settings") {
            navigate("/settings/storage"); // navigate to status settings by default
        }
    }, [location.pathname, navigate]);

    return (
        <>
            <TopLevelBreadcrumbs pages={pages} />

            <SecondaryLevelBreadcrumbs pages={secondaryNavigation} />

            {/* this will render the current route's component */}
            <Outlet />
        </>
    );
}
