import { createApp } from "vue";

import App from "./App.vue";
import { i18n } from "./i18n";
import "./ui/styles/index.css";

const platform = navigator.userAgent.includes("Mac OS X") ? "macos" : "other";
document.documentElement.dataset.platform = platform;

createApp(App).use(i18n).mount("#app");
