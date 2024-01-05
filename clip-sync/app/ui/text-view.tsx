import { FormEvent, useState } from 'react';
import { Entry, SearchParam, search } from '../lib/api';
import { init } from 'next/dist/compiled/webpack/webpack';

function EntryView(entry: Entry) {
    return (
        <li key={`${entry.source}:${entry.timestamp}`}>
            <pre>{entry.text}</pre>
        </li>
    )
}

function History(entries: Entry[]) {
    return (
        <ul>
            {entries.map((entry) => EntryView(entry))}
        </ul>
    )
}

export function SearchableTextHistory() {
    let initParam: SearchParam = {
        text: '',
        sources: [],
        begin: undefined,
        end: undefined,
        start: 0,
        size: 100,
        skip: 0,
    };
    let initResult: Entry[] = [];
    let [param, setParam] = useState(initParam);
    let [result, setResult] = useState(initResult);

    var initTimerId: any | null = null;
    let [timerId, setTimerId] = useState(initTimerId);

    function onSearch(event: FormEvent<HTMLInputElement>) {
        if (timerId) {
            clearTimeout(timerId);
        }
        setTimerId(setTimeout(() => {
            let text = (event.target as HTMLInputElement).value;
            let p = param;
            p.text = text;
            setParam(p);
            search(param, (r) => {
                setResult(r);
            });
        }, 500));
    }

    return (
        <div>
            <input id="search-text" type="text" onInput={onSearch} placeholder='Search text' />
            <div>
                {History(result)}
            </div>
        </div>
    )
}