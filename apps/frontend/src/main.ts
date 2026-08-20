import { createApp } from "vue";

import "@dsh-desktop/ui/styles";
import "element-plus/es/components/icon/style/css";
import "element-plus/es/components/input-number/style/css";
import "element-plus/es/components/select/style/css";
import "element-plus/es/components/slider/style/css";

import App from "./App.vue";
import { i18n } from "./i18n";
import "./styles/element-plus-theme.css";

const platform = navigator.userAgent.includes("Mac OS X") ? "macos" : "other";
document.documentElement.dataset.platform = platform;

createApp(App).use(i18n).mount("#app");
