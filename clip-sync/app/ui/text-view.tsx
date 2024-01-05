import { FormEvent, useEffect, useState } from 'react';
import { Entry, SearchParam, search } from '../lib/api';
import { init } from 'next/dist/compiled/webpack/webpack';
import Search from 'antd/es/input/Search';
import { match } from 'assert';
import Button, { isString } from 'antd/es/button';
import { on } from 'events';
import { Divider, Space, Tag, message } from 'antd';
import { CopyTwoTone } from '@ant-design/icons';
import { MessageInstance } from 'antd/es/message/interface';

function EntryView(entry: Entry, messageApi: MessageInstance) {
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
        <li key={`${entry.source}:${entry.timestamp}`}>
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
    return (
        <ul>
            {entries.map((entry) => EntryView(entry, messageApi))}
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
    let initResult: Entry[] = [];
    let [param, setParam] = useState(initParam);
    let [result, setResult] = useState(initResult);

    var initTimerId: any | null = null;
    let [timerId, setTimerId] = useState(initTimerId);

    useEffect(() => {
        search(initParam, (r) => {
            setResult(r);
        });
    }, []);

    function onInput(value: string | FormEvent<HTMLInputElement>) {
        if (timerId) {
            clearTimeout(timerId);
        }
        setTimerId(setTimeout(() => {
            let p = param;
            if (isString(value)) {
                p.text = value;
            } else {
                p.text = (value.target as HTMLInputElement).value;
            }
            setParam(p);
            search(param, (r) => {
                setResult(r);
            });
        }, 500));
    }

    return (
        <div>
            {contextHolder}
            <Search placeholder="input search text" onSearch={onInput} allowClear enterButton onInput={onInput} autoFocus />
            <Divider />
            <div>
                {History(result, messageApi)}
            </div>
        </div>
    )
}