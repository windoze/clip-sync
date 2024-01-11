export type Entry = {
    source: string;
    text: string;
    imageurl: string;
    timestamp: number;
};

export type SearchParam = {
    text: string;
    sources: string[];
    begin: number | undefined;
    end: number | undefined;
    size: number | undefined;
    skip: number | undefined;
};

export type SearchResult = {
    total: number;
    skip: number;
    data: Entry[];
};

const API_URL = import.meta.env.VITE_API_ROOT;

export function getApiRoot(): string {
    let apiRoot = API_URL ? API_URL : "/api/";
    if (!apiRoot.endsWith("/")) {
        apiRoot = apiRoot + "/";
    }
    const url = new URL(`${apiRoot}`, window.location.origin);
    return url.toString();
}

export async function search(param: SearchParam): Promise<SearchResult> {
    const { text, sources, begin, end, size, skip } = param;
    const url = new URL(`${getApiRoot()}query`, window.location.origin);
    if (text) url.searchParams.append("q", text);
    if (sources && sources.length > 0) url.searchParams.append("from", sources.join(","));
    if (begin) url.searchParams.append("begin", begin.toString());
    if (end) url.searchParams.append("end", end.toString());
    if (size) url.searchParams.append("size", size.toString());
    if (skip) url.searchParams.append("skip", skip.toString());
    console.log(url.toString());
    const res = await fetch(url);
    if (res.ok) {
        return await res.json();
    } else {
        return { total: 0, skip: 0, data: [] };
    }
}

export async function getDeviceList(): Promise<string[]> {
    const url = new URL(`${getApiRoot()}device-list`);
    const res = await fetch(url);
    if (res.ok) {
        const json = await res.json();
        return (json as string[]).filter((device) => !device.startsWith("$"));
    } else {
        return [];
    }
}

export async function getImageCollection(name: string): Promise<string[]> {
    const url = new URL(`${getApiRoot()}collection/${name}`);
    const res = await fetch(url);
    if (res.ok) {
        const json = await res.json();
        let images = json as string[];

        return images.map((image) => {
            return `${getApiRoot()}images/${image}`;
        });
    } else {
        return [];
    }
}

export class WebSocketComponent {
    private url: string = "";
    private socket: WebSocket | undefined;
    private listeners: ((data: any) => void)[] = [];

    constructor() {
        let url = new URL(`${getApiRoot()}clip-sync/$utilities`);
        url.protocol = url.protocol.replace("https", "wss");
        url.protocol = url.protocol.replace("http", "ws");
        this.url = url.toString();

        this.connect();
    }

    public addListener(listener: (data: any) => void) {
        this.listeners.push(listener);
    }

    public removeListener(listener: (data: any) => void) {
        this.listeners = this.listeners.filter((l) => l !== listener);
    }

    public send(data: any) {
        if (this.socket) {
            this.socket.send(data);
        }
    }

    private connect() {
        this.socket = new WebSocket(this.url);
        this.socket.onerror = () => {
            console.log("WebSocket error, closing...");
            this.socket?.close();
        }
        this.socket.onclose = () => {
            console.log("WebSocket closed, retrying in 1 second...");
            setTimeout(() => {
                this.connect();
            }, 1000);
        }
        console.log(`WebSocket to ${this.url} created`);
        this.socket.addEventListener("message", (event) => {
            let msg = JSON.parse(event.data + "");
            if (msg.source.startsWith('$')) {
                return;
            }
            this.listeners.forEach((listener) => {
                listener(msg);
            });
        });
    }
}

export const webSocketComponent = new WebSocketComponent();