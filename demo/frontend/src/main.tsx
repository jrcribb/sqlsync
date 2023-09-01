import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import "./index.css";

const DEMO_REDUCER_URL = new URL(
  "../../../target/wasm32-unknown-unknown/debug/demo_reducer.wasm",
  import.meta.url
);

import init, { JournalId, RandomJournalId } from "sqlsync-worker";
import wasmUrl from "sqlsync-worker/sqlsync.wasm?url";
import workerUrl from "sqlsync-worker/worker.js?url";

const COORDINATOR_URL = "localhost:8787";

// const newDocument = async () => {
//   const response = await fetch(`${location.protocol}//${COORDINATOR_URL}/new`, {
//     method: "POST",
//   });
//   return (await response.text()) as JournalId;
// };
//const DOC_ID = await newDocument();
const DOC_ID = "6cBMtRnfvn5B5GSP5oxFyuXh9ExxrjgQ93zaBhnpEVqR" as JournalId;

const TIMELINE_ID = RandomJournalId();

const sqlsync = await init(workerUrl, wasmUrl, COORDINATOR_URL);

await sqlsync.open(DOC_ID, TIMELINE_ID, DEMO_REDUCER_URL);

type Mutation =
  | {
      tag: "InitSchema";
    }
  | {
      tag: "Incr";
      value: number;
    }
  | {
      tag: "Decr";
      value: number;
    };

const mutate = (mutation: Mutation) => {
  const buf = JSON.stringify(mutation);
  const bytes = new TextEncoder().encode(buf);
  return sqlsync.mutate(DOC_ID, bytes);
};

await mutate({ tag: "InitSchema" });

window.incr = async (value = 1) => {
  await mutate({ tag: "Incr", value });
};
window.decr = async (value = 1) => {
  await mutate({ tag: "Decr", value });
};
window.query = (query = "select * from counter") => {
  return sqlsync.query(DOC_ID, query, []);
};

// TODO: Figure out how to make sure errors are propagated out of the worker

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
