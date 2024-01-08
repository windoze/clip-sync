import { FormEvent, useEffect, useState } from 'react';
import { Entry, SearchParam, SearchResult, getApiRoot, getDeviceList, search } from '../lib/api';
import { Button, Divider, Empty, Input, Pagination, DatePicker, Space, Spin, Tag, Tooltip, Select, Alert } from 'antd';
import { CopyTwoTone, SearchOutlined, SettingFilled } from '@ant-design/icons';
import { MessageInstance } from 'antd/es/message/interface';
import { useTranslation } from 'react-i18next';

const { RangePicker } = DatePicker;

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

export function EntryView(entry: Entry, messageApi: MessageInstance, relTime: boolean = true, t: any) {
    function onCopy() {
        navigator.clipboard.writeText(entry.text);
        messageApi.open({
            type: 'success',
            content: t('copiedMessage'),
            duration: 3,
        });
    }
    let time = new Date(entry.timestamp * 1000);
    let timeStrTip = time.toLocaleString();
    let timeStr = getRelativeTimeString(time);
    if (!relTime) {
        timeStr = time.toLocaleString();
    }
    let source = entry.source;
    if (source.startsWith('$')) {
        source = t('systemUtil');
    }
    if (entry.text && entry.text.length > 0) {
        return (
            <div className="relative">
                <Tooltip placement="topRight" title={t('copyTextToClipboard')}> <Button className="absolute flex flex-row  top-0 right-0 p-2" onClick={onCopy} ><CopyTwoTone twoToneColor="#87b7f3" /></Button></Tooltip>
                <pre><code className="language-css">{entry.text}</code></pre>
                <div className="flex flex-row">
                    <Tag color="blue">{source}</Tag>
                    <Tooltip placement="bottomLeft" title={timeStrTip}><Tag color="green">{timeStr}</Tag></Tooltip>
                </div>
            </div>
        )
    } else if (entry.imageurl && entry.imageurl.length > 0) {
        let imageUrl = `${getApiRoot()}images/${entry.imageurl}`;
        return (
            <div className="relative" style={{ textAlign: 'left' }}>
                <a href={imageUrl} target="_blank"><picture><img src={imageUrl} alt={imageUrl} width={100} height={100} /></picture></a>
                <Tag color="blue">{source}</Tag>
                <Tooltip placement="bottomLeft" title={timeStrTip}><Tag color="green">{timeStr}</Tag></Tooltip>
            </div>
        )
    } else {
        return (<div>
            <Alert message={t("unsupportedEntry")} type="warning" showIcon />
            <Tag color="blue">{source}</Tag>
            <Tooltip placement="bottomLeft" title={timeStrTip}><Tag color="green">{timeStr}</Tag></Tooltip>
        </div>
        );
    }
}

function History(entries: SearchResult, messageApi: MessageInstance, t: any) {
    if (entries.total < 0) {
        return <Spin />
    }
    else if (entries.total == 0) {
        return <Empty />
    }
    function item(entry: Entry, index: number) {
        return (
            console.log('entry', index, entry),
            <li key={index}>
                {EntryView(entry, messageApi, true, t)}
            </li>
        )
    }
    return (
        <ul>
            {entries.data.map((entry, index) => item(entry, index + entries.skip))}
        </ul>
    )
}

function CountText(count: number) {
    const { t } = useTranslation();

    if (count < 0) {
        return <Divider>Searching...</Divider>;
    }
    return (
        <Divider>{t('resultCount', { count: count })}</Divider>
    )
}

const initResult: SearchResult = { data: [], skip: 0, total: -1 };
const initDeviceList: string[] = [];
const initTimerId: any | null = null;
const initParam: SearchParam = {
    text: '',
    sources: [],
    begin: undefined,
    end: undefined,
    size: 20,
    skip: 0,
};

export function SearchableTextHistory(messageApi: MessageInstance) {
    const { t } = useTranslation();

    let [param, setParam] = useState(initParam);
    let [result, setResult] = useState(initResult);
    let [deviceList, setDeviceList] = useState(initDeviceList);
    let [timerId, setTimerId] = useState(initTimerId);
    var [settingsVisible, setSettingsVisible] = useState(false);

    async function s(p: SearchParam) {
        console.log('searching', p);
        let skip = p.skip ? p.skip : 0;
        let r = await search(p);
        if ((r.total > 0) && (r.total <= skip)) {
            // skip too large, reset to 0 and search again
            p.skip = 0;
            r = await search(p);
        }
        setParam(p);
        setResult(r);
    }

    useEffect(() => {
        setResult(initResult);
        s(initParam).then((r) => {
            return r;
        });
        getDeviceList().then((r) => {
            setDeviceList(r);
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
            s(p).then((r) => {
                return r;
            });
        }, 500));
    }

    function onPagerChange(page: number, pageSize?: number) {
        let p = { ...param };
        p.size = pageSize ? pageSize : 20;
        p.skip = (page - 1) * p.size;
        setParam(p);
        s(p).then((r) => {
            return r;
        });
    }

    function Pager() {
        if (result.total <= 0) {
            return <div />
        }
        return (
            <Pagination total={result.total} pageSize={20} showSizeChanger={false} onChange={onPagerChange} />
        )
    }

    function onSettingsClick() {
        let v = !settingsVisible;
        setSettingsVisible(v);
        if (v) {
            getDeviceList().then((r) => {
                setDeviceList(r);
                return r;
            });
        } else {
            console.log('settings closed');
            let p = { ...param };
            p.sources = [];
            p.begin = undefined;
            p.end = undefined;
            s(p).then((r) => {
                return r;
            });
        }
    }

    function SettingsPane() {
        const handleDeviceChange = (value: string[]) => {
            let p = { ...param, sources: value };
            setParam(p);
            s(p).then((r) => {
                return r;
            });
        };

        const onRangeChange = (dates: any, dateStrings: [string, string]) => {
            console.log(dates, dateStrings);
            if (dates === null || dates.length == 0) {
                dates = [undefined, undefined];
            }
            let beginTimestamp = dates[0] ? dates[0].unix() : undefined;
            let endTimestamp = dates[1] ? dates[1].unix() : undefined;
            let p = { ...param, begin: beginTimestamp, end: endTimestamp };
            s(p).then((r) => {
                return r;
            });
        }

        if (!settingsVisible) {
            return <div />
        }
        let options = deviceList.map((v) => { return { label: v, value: v, emoji: '🖥️', } });
        return (
            <div className="flex flex-row justify-between spaced">
                <Select
                    mode="multiple"
                    style={{ width: '100%' }}
                    placeholder={t("deviceListPlaceholder")}
                    onChange={handleDeviceChange}
                    options={options}
                    optionRender={(option) => (
                        <Space>
                            <span role="img" aria-label={option.data.label}>
                                {option.data.emoji}
                            </span>
                            {option.data.value}
                        </Space>
                    )}
                />
                <Divider type='vertical' ></Divider>
                <RangePicker showTime placeholder={[t('timeRangeStart'), t('timeRangeEnd')]} changeOnBlur={true} style={{ color: '#888888' }} onCalendarChange={onRangeChange} />
            </div>
        )
    }

    return (
        <div>
            <div className="flex flex-row justify-between">
                <Input placeholder={t("searchPlaceholder")} allowClear onChange={onInput} autoFocus addonBefore={<SearchOutlined />} />
                <Button type='primary' ghost icon={<SettingFilled />} onClick={onSettingsClick} />
            </div>
            {SettingsPane()}
            {CountText(result.total)}
            <div>
                {History(result, messageApi, t)}
            </div>
            {Pager()}
        </div>
    )
}