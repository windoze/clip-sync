import { useEffect, useState } from 'react';
import { Divider, Menu, MenuProps } from 'antd';
import ImageGallery, { ReactImageGalleryItem } from "react-image-gallery";
import { getDeviceList, getImageCollection } from '../lib/api';
import { DesktopOutlined } from '@ant-design/icons';

const initDeviceList: MenuProps['items'] = [];

export function DeviceGallery(imageList: ReactImageGalleryItem[]) {
    return (
        <div>
            <ImageGallery items={imageList} showPlayButton={false} showIndex={true} showFullscreenButton={false} />
        </div>
    );
}

export function ImageView() {
    let [deviceList, setDeviceList] = useState(initDeviceList);
    const [current, setCurrent] = useState('');
    let [imageList, setImageList] = useState<ReactImageGalleryItem[]>([]);
    const onClick: MenuProps['onClick'] = (e) => {
        console.log('click ', e);
        setCurrent(e.key);
        getImageCollection(e.key).then((r) => {
            let images = r.map((d) => {
                return {
                    original: d,
                    thumbnail: d,
                };
            });
            setImageList(images);
            return r;
        });
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
            getImageCollection(devices[0].key).then((r) => {
                let images = r.map((d) => {
                    return {
                        original: d,
                        thumbnail: d,
                    };
                });
                setImageList(images);
                return r;
            });
            return r;
        });
    }, []);

    return (
        <div style={{ minHeight: '100vh', verticalAlign: 'top' }}>
            <Menu onClick={onClick} selectedKeys={[current]} mode="horizontal" items={deviceList} />
            <Divider type='horizontal' ></Divider>
            {DeviceGallery(imageList)}
        </div>
    );
}