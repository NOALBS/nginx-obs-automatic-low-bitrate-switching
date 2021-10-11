export class Ws {
  constructor(address, user_config) {
    this.address = address;
    this.config = user_config;

    this.messageId = 0;
    this.requests = {};

    this.connect();
  }

  connect() {
    this.ws = new WebSocket(this.address);

    this.ws.onopen = this.onOpen.bind(this);
    this.ws.onmessage = this.onMessage.bind(this);
    this.ws.onclose = this.onClose.bind(this);
  }

  async onOpen() {
    if (this.ws !== null && this.ws.readyState === 1) {
      console.log("Successfully Connected to WS");
    }
  }

  onClose() {
    this.reconnect();
  }

  close() {
    if (this.ws) {
      this.ws.close();
    }
  }

  reconnect() {
    console.log(`Disconnected from WS trying to reconnect in 5 seconds`);

    setTimeout(() => {
      console.log("Reconnecting to WS...");
      this.connect();
    }, 5000);
  }

  onMessage(message) {
    const data = JSON.parse(message.data);

    if (data.event) {
      console.log("got event:", data.event, data);
      if (data.event == "prefixChanged") {
        this.config.update((o) => {
          o.config.chat.prefix = data.data.prefix;
          return o;
        });
      }
    }

    if (data.nonce) {
      let res = this.requests[data.nonce];
      res(data.data);

      delete this.requests[data.nonce];

      return;
    }
  }

  async sendReq(message) {
    if (this.ws.readyState <= 1) {
      message.nonce = `${this.messageId++}`;

      this.ws.send(JSON.stringify(message));

      return await new Promise((resolve, reject) => {
        this.requests[message.nonce] = resolve;

        setTimeout(() => {
          delete this.requests[message.nonce];
          reject(new Error("Request took too long"));
        }, 1000 * 5);
      });
    }
  }
}
