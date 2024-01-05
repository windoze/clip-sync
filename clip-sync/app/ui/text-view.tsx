import { FormEvent, useEffect, useState } from 'react';
import { Entry, SearchParam, SearchResult, search } from '../lib/api';
import { Button, Divider, Input, Space, Tag, Tooltip, message } from 'antd';
import { CopyTwoTone, SearchOutlined } from '@ant-design/icons';
import { MessageInstance } from 'antd/es/message/interface';

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
    // let timeStrTip = `${time.getFullYear()}-${time.getMonth() + 1}-${time.getDate()} ${time.getHours()}:${time.getMinutes()}:${time.getSeconds()}`;
    let timeStrTip = time.toLocaleString();
    let timeStr = getRelativeTimeString(time);
    return (
        <li key={index}>
            <div className="relative">
                <Button className="absolute flex flex-row  top-0 right-0 p-2" onClick={onCopy} ><CopyTwoTone twoToneColor="#87b7f3" /></Button>
                <pre><code className="language-css">{entry.text}</code></pre>
                <Space size={[0, 2]} wrap>
                    <Tag color="blue">{entry.source}</Tag>
                    <Tooltip placement="bottomLeft" title={timeStrTip}><Tag color="green">{timeStr}</Tag></Tooltip>
                </Space>
                <Divider />
            </div>
        </li>
    )
}

function History(entries: Entry[], messageApi: MessageInstance) {
    return (
        <ul>
            {entries.map((entry, index) => EntryView(entry, messageApi, index))}
        </ul>
    )
}

function CountText(count: number) {
    if (count < 0) {
        return <Divider >Searching...</Divider>;
    }
    return (
        <Divider orientation="right">Found {count} entries</Divider>
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
        size: 100,
        skip: 0,
    };
    let [param, setParam] = useState(initParam);
    let initEntries: Entry[] = [];
    let [entries, setEntries] = useState(initEntries);

    var initTimerId: any | null = null;
    let [timerId, setTimerId] = useState(initTimerId);
    let [count, setCount] = useState(0);

    useEffect(() => {
        setCount(-1);
        search(initParam, (r) => {
            mergeResult(r);
        });
    }, []);

    function mergeResult(result: SearchResult) {
        // Merge result with existing entries
        let e = [
            ...entries.slice(0, result.skip),
            ...result.data,
        ];
        setEntries(e);
        setCount(result.total);
    }

    function onInput(value: FormEvent<HTMLInputElement>) {
        if (timerId) {
            clearTimeout(timerId);
        }
        setTimerId(setTimeout(() => {
            setCount(-1);
            let p = param;
            p.text = (value.target as HTMLInputElement).value;
            setParam(p);
            search(param, (r) => {
                mergeResult(r);
            });
        }, 500));
    }

    return (
        <div>
            {contextHolder}
            <Input placeholder="input search text" allowClear onChange={onInput} autoFocus addonBefore={<SearchOutlined />} />
            {CountText(count)}
            <div>
                {History(entries, messageApi)}
            </div>
        </div>
    )
}