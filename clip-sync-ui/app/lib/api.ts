import exp from "constants";

export type Entry = {
    source: string;
    text: string;
    timestamp: number;
};

export type SearchParam = {
    text: string;
    sources: string[];
    begin: number | undefined;
    end: number | undefined;
    start: number | undefined;
    size: number | undefined;
    skip: number | undefined;
};

export type SearchResult = {
    total: number;
    skip: number;
    data: Entry[];
};

const API_URL = process.env.NEXT_PUBLIC_API_ROOT;

export async function search(param: SearchParam): Promise<SearchResult> {
    let apiRoot = API_URL ? API_URL : "/api/";
    if (!apiRoot.endsWith("/")) {
        apiRoot = apiRoot + "/";
    }

    const { text, sources, begin, end, start, size, skip } = param;
    const url = new URL(`${apiRoot}query`, window.location.origin);
    if (text) url.searchParams.append("q", text);
    if (sources && sources.length > 0) url.searchParams.append("from", sources.join(","));
    if (begin) url.searchParams.append("begin", begin.toString());
    if (end) url.searchParams.append("end", end.toString());
    if (start) url.searchParams.append("start", start.toString());
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
    let apiRoot = API_URL ? API_URL : "/api/";
    if (!apiRoot.endsWith("/")) {
        apiRoot = apiRoot + "/";
    }

    const url = new URL(`${apiRoot}device-list`, window.location.origin);
    const res = await fetch(url);
    if (res.ok) {
        const json = await res.json();
        return json as string[];
    } else {
        return [];
    }
}

export async function getImageCollection(name: string): Promise<string[]> {
    let apiRoot = API_URL ? API_URL : "/api/";
    if (!apiRoot.endsWith("/")) {
        apiRoot = apiRoot + "/";
    }

    const url = new URL(`${apiRoot}collection/${name}`, window.location.origin);
    const res = await fetch(url);
    if (res.ok) {
        const json = await res.json();
        let images = json as string[];

        return images.map((image) => {
            return `${apiRoot}images/${image}`;
        });
    } else {
        return [];
    }
}