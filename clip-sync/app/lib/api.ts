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

const API_URL = "https://clip.0d0a.com:23000/api";

export function search(param: SearchParam, callback: ((result: Entry[]) => any)) {
    const { text, sources, begin, end, start, size, skip } = param;
    const url = new URL(`${API_URL}/query`);
    if (text) url.searchParams.append("q", text);
    if (sources && sources.length > 0) url.searchParams.append("sources", sources.join(","));
    if (begin) url.searchParams.append("begin", begin.toString());
    if (end) url.searchParams.append("end", end.toString());
    if (start) url.searchParams.append("start", start.toString());
    if (size) url.searchParams.append("size", size.toString());
    if (skip) url.searchParams.append("skip", skip.toString());
    console.log(url.toString());
    const res = fetch(url).then(res => {
        if (res.ok) {
            res.json().then(json => {
                callback(json as Entry[]);
            });
        } else {
            callback([]);
        }
    });
}

export async function getDeviceList(): Promise<string[]> {
    const url = new URL(`${API_URL}/device-list`);
    const res = await fetch(url);
    if (res.ok) {
        const json = await res.json();
        return json as string[];
    } else {
        return [];
    }
}