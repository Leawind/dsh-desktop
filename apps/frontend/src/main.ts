import { createApp } from "vue";

import "@dsh-desktop/ui/styles";

import App from "./App.vue";
import { i18n } from "./i18n";

const platform = navigator.userAgent.includes("Mac OS X") ? "macos" : "other";
document.documentElement.dataset.platform = platform;

createApp(App).use(i18n).mount("#app");
