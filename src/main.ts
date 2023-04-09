import init from "./wasm";

import { initializeApp } from "@firebase/app";
import { firebaseConfig } from "./firebaseConfig";

import "./style.css";

try {
  initializeApp(firebaseConfig);
} catch (err) {
  console.error("failed to initialize app", err);
}

init();
