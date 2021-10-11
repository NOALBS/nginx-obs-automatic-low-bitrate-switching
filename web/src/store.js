import { derived, writable } from "svelte/store";

const WS_URL = `ws://${window.location.hostname}:3000/ws`;

import { Ws } from "./ws.js";

export const user = writable({});
export const ws = new Ws(WS_URL, user);

export const sendWsMessage = (message) => {
  return ws.sendReq(message);
};

export const isLoggedIn = derived(user, ($user) => $user?.config != null);
