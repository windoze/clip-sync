import { FormEvent, useEffect, useState } from 'react';
import { Entry, SearchParam, SearchResult, search } from '../lib/api';
import { Button, Divider, Empty, Input, Pagination, Space, Spin, Tag, Tooltip, message } from 'antd';
import { CopyTwoTone, SearchOutlined, SettingOutlined } from '@ant-design/icons';
import { MessageInstance } from 'antd/es/message/interface';
import { pages } from 'next/dist/build/templates/app-page';

export function getRelativeTimeString(
    date: Date | number,
    lang = navigator.language
): string {
    // Allow dates or times to be passed
    const timeMs = typeof date === "number" ? date : date.getTime();

    // Get the amount of seconds between the given date and now
    const deltaSeconds = Math.round((timeMs - Date.now()) / 1000);

    // Array representing one minute, hour, day, week, month, etc in seconds
    const cutoffs = [60, 3600, 86400, 86400 * 7, 86400 * 30, 86400 * 365, Infinity];

    // Array equivalent to the above but in the string representation of the units
    const units: Intl.RelativeTimeFormatUnit[] = ["second", "minute", "hour", "day", "week", "month", "year"];

    // Grab the ideal cutoff unit
    const unitIndex = cutoffs.findIndex(cutoff => cutoff > Math.abs(deltaSeconds));

    // Get the divisor to divide from the seconds. E.g. if our unit is "day" our divisor
    // is one day in seconds, so we can divide our seconds by this to get the # of days
    const divisor = unitIndex ? cutoffs[unitIndex - 1] : 1;

    // Intl.RelativeTimeFormat do its magic
    const rtf = new Intl.RelativeTimeFormat(lang,);
    return rtf.format(Math.floor(deltaSeconds / divisor), units[unitIndex]);
}

function EntryView(entry: Entry, messageApi: MessageInstance, index: number) {
    function onCopy() {
        navigator.clipboard.writeText(entry.text);
        messageApi.open({
            type: 'success',
            content: 'Copied to clipboard',
            duration: 3,
        });
    }
    let time = new Date(entry.timestamp * 1000);
    let timeStrTip = time.toLocaleString();
    let timeStr = getRelativeTimeString(time);
    return (
        <li key={index}>
            <div className="relative">
                <Tooltip placement="topRight" title="Copy to clipboard"><Button className="absolute flex flex-row  top-0 right-0 p-2" onClick={onCopy} ><CopyTwoTone twoToneColor="#87b7f3" /></Button></Tooltip>
                <pre><code className="language-css">{entry.text}</code></pre>
                <Tag color="blue">{entry.source}</Tag>
                <Tooltip placement="bottomLeft" title={timeStrTip}><Tag color="green">{timeStr}</Tag></Tooltip>
            </div>
            {/* <Divider /> */}
        </li>
    )
}

function History(entries: SearchResult, messageApi: MessageInstance) {
    if (entries.total < 0) {
        return <Spin />
    }
    else if (entries.total == 0) {
        return <Empty />
    }
    return (
        <ul>
            {entries.data.map((entry, index) => EntryView(entry, messageApi, index + entries.skip))}
        </ul>
    )
}

function CountText(count: number) {
    if (count < 0) {
        return <Divider orientation="left">Searching...</Divider>;
    }
    return (
        <Divider orientation="left">Found {count} entries</Divider>
    )
}

export function SearchableTextHistory() {
    const [messageApi, contextHolder] = message.useMessage();
    let initParam: SearchParam = {
        text: '',
        sources: [],
        begin: undefined,
        end: undefined,
        start: 0,
        size: 20,
        skip: 0,
    };
    let [param, setParam] = useState(initParam);

    let initResult: SearchResult = { data: [], skip: 0, total: -1 };
    let [result, setResult] = useState(initResult);

    var initTimerId: any | null = null;
    let [timerId, setTimerId] = useState(initTimerId);

    async function s() {
        let r = await search(param);
        setResult(r);
    }

    useEffect(() => {
        setResult(initResult);
        s().then((r) => {
            return r;
        });
    }, []);

    function onInput(value: FormEvent<HTMLInputElement>) {
        if (timerId) {
            clearTimeout(timerId);
        }
        setTimerId(setTimeout(() => {
            setResult(initResult);
            let p = param;
            p.text = (value.target as HTMLInputElement).value;
            setParam(p);
            s().then((r) => {
                console.log(r);
                return r;
            });
        }, 500));
    }

    function onPagerChange(page: number, pageSize?: number) {
        let p = param;
        p.skip = (page - 1) * (pageSize || 20);
        p.size = pageSize || 20;
        setParam(p);
        s().then((r) => {
            console.log(r);
            return r;
        });
    }

    function Pager() {
        if (result.total <= 0) {
            return <div />
        }
        return (
            <Pagination total={result.total} pageSize={param.size} onChange={onPagerChange} />
        )
    }

    return (
        <div>
            {contextHolder}
            <Input placeholder="input search text" allowClear onChange={onInput} autoFocus addonBefore={<SearchOutlined />} addonAfter={<SettingOutlined />} />
            {CountText(result.total)}
            <div>
                {History(result, messageApi)}
            </div>
            {Pager()}
        </div>
    )
}