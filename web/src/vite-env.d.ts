/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Absolute base URL of the MPGS server; empty string means same-origin. */
  readonly VITE_MPGS_API_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
