#!/usr/bin/env python3
import asyncio
import base64
import hashlib
import json
import time
from urllib.parse import parse_qs, urlparse

HOST = "127.0.0.1"
PORT = 8765
GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


def websocket_accept(key):
    digest = hashlib.sha1((key + GUID).encode()).digest()
    return base64.b64encode(digest).decode()


def encode_frame(message):
    payload = message.encode()
    length = len(payload)

    if length < 126:
        header = bytes([0x81, length])
    elif length < 65536:
        header = bytes([0x81, 126]) + length.to_bytes(2, "big")
    else:
        header = bytes([0x81, 127]) + length.to_bytes(8, "big")

    return header + payload


async def handle_client(reader, writer):
    request = await reader.readuntil(b"\r\n\r\n")
    header_text = request.decode(errors="ignore")
    lines = header_text.split("\r\n")
    path = lines[0].split(" ")[1]
    headers = {}

    for line in lines[1:]:
        if ":" in line:
            key, value = line.split(":", 1)
            headers[key.lower()] = value.strip()

    key = headers.get("sec-websocket-key")
    if not key:
        writer.close()
        await writer.wait_closed()
        return

    response = (
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Accept: {websocket_accept(key)}\r\n"
        "\r\n"
    )
    writer.write(response.encode())
    await writer.drain()

    query = parse_qs(urlparse(path).query)
    feed = query.get("feed", ["feed1"])[0]
    print(f"client connected feed={feed}")

    counter = 0
    try:
        while True:
            bitrate = 300 if (counter // 20) % 2 else 6000
            message = {
                "type": "stats",
                "timestamp": int(time.time() * 1000),
                "streamId": f"publish/live/{feed}",
                "feed": feed,
                "bitrate": bitrate,
                "packetLoss": 0,
                "rtt": 80,
                "connected": True,
            }
            writer.write(encode_frame(json.dumps(message)))
            await writer.drain()
            counter += 1
            await asyncio.sleep(0.25)
    except (ConnectionError, asyncio.CancelledError):
        pass
    finally:
        writer.close()
        await writer.wait_closed()
        print(f"client disconnected feed={feed}")


async def main():
    server = await asyncio.start_server(handle_client, HOST, PORT)
    print(f"mock WebSocket stats server listening on ws://{HOST}:{PORT}/ws-stats?feed=feed1")

    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
