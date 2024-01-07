import { FormEvent, useEffect, useState } from 'react';
import { Divider, Menu, MenuProps } from 'antd';
import ImageGallery, { ReactImageGalleryItem } from "react-image-gallery";
import { getDeviceList, getImageCollection } from '../lib/api';
import { DesktopOutlined } from '@ant-design/icons';

const initDeviceList: MenuProps['items'] = [];
const initImageLit: ReactImageGalleryItem[] = [];

export function DeviceGallery(name: string) {
    let [imageList, setImageList] = useState(initImageLit);
    useEffect(() => {
        getImageCollection(name).then((r) => {
            let images = r.map((d) => {
                return {
                    original: d,
                    thumbnail: d,
                };
            });
            setImageList(images);
            return r;
        });
    });
    return (
        <div>
            <ImageGallery items={imageList} showPlayButton={false} showIndex={true} showFullscreenButton={false} />
        </div>
    );
}

export function ImageView() {
    let [deviceList, setDeviceList] = useState(initDeviceList);
    const [current, setCurrent] = useState('');
    const onClick: MenuProps['onClick'] = (e) => {
        console.log('click ', e);
        setCurrent(e.key);
    };

    useEffect(() => {
        getDeviceList().then((r) => {
            let devices = r.map((d) => {
                return {
                    key: d,
                    label: d,
                    icon: <DesktopOutlined />,
                };
            });
            setDeviceList(devices);
            setCurrent(devices[0].key);
            return r;
        });
    }, []);

    return (
        <div>
            <Menu onClick={onClick} selectedKeys={[current]} mode="horizontal" items={deviceList} />
            <Divider type='horizontal' ></Divider>
            {DeviceGallery(current)}
        </div>
    );
}