import { FormEvent, useEffect, useState } from 'react';
import { Entry, SearchParam, SearchResult, search } from '../lib/api';
import { init } from 'next/dist/compiled/webpack/webpack';
import Search from 'antd/es/input/Search';
import { match } from 'assert';
import Button, { isString } from 'antd/es/button';
import { on } from 'events';
import { Divider, Input, Space, Tag, message } from 'antd';
import { CopyTwoTone, SearchOutlined } from '@ant-design/icons';
import { MessageInstance } from 'antd/es/message/interface';

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
    let timeStr = `${time.getFullYear()}-${time.getMonth() + 1}-${time.getDate()} ${time.getHours()}:${time.getMinutes()}:${time.getSeconds()}`;
    return (
        <li key={index}>
            <div className="relative">
                <Button className="absolute flex flex-row  top-0 right-0 p-2" onClick={onCopy} ><CopyTwoTone twoToneColor="#87b7f3" /></Button>
                <pre><code className="language-css">{entry.text}</code></pre>
                <Space size={[0, 2]} wrap>
                    <Tag color="blue">{entry.source}</Tag>
                    <Tag color="green">{timeStr}</Tag>
                </Space>
                <Divider />
            </div>
        </li>
    )
}

function History(entries: Entry[], messageApi: MessageInstance) {
    console.log("YYYYYY", entries);
    return (
        <ul>
            {entries.map((entry, index) => EntryView(entry, messageApi, index))}
        </ul>
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

    useEffect(() => {
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
    }

    function onInput(value: FormEvent<HTMLInputElement>) {
        if (timerId) {
            clearTimeout(timerId);
        }
        setTimerId(setTimeout(() => {
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
            <Divider />
            <div>
                {History(entries, messageApi)}
            </div>
        </div>
    )
}