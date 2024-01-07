'use client'

import '../public/antd.min.css';
import '../app/globals.css';
import { Col, Flex, Layout, Row, Tabs, message } from 'antd';
import { EntryView, SearchableTextHistory } from './ui/text-view';
import { GithubOutlined } from '@ant-design/icons';
import { CSSProperties, useEffect, useState } from 'react';
import Image from 'next/image';
import { ImageView } from './ui/image-view';
import { UtilsView } from './ui/utils-view';
import { webSocketComponent } from './lib/api';
import { MessageInstance } from 'antd/es/message/interface';

const { Header, Footer, Content } = Layout;


function ActionItem(msg: any) {
  return
}

export default function Home() {
  const [messageApi, contextHolder] = message.useMessage();
  let [actions, setActions] = useState<any[]>([]);

  function messageBubbleHandler(msg: any) {
    console.log("Page:", msg);
    let time = new Date(msg.timestamp * 1000);
    messageApi.open({
      type: 'info',
      content: `${msg.source} copied '${msg.text}'`,
      duration: 3,
    });
    setActions([
      {
        // label: time.toLocaleTimeString(),
        children: EntryView(msg, messageApi, false),
      },
      ...actions,
    ]);
  }

  useEffect(() => {
    // Did mount
    webSocketComponent.addListener(messageBubbleHandler);
    return () => {
      // Will unmount
      webSocketComponent.removeListener(messageBubbleHandler);
    }
  });

  const items = [
    {
      key: '1',
      label: 'Text',
      children: SearchableTextHistory(messageApi),
    },
    {
      key: '2',
      label: 'Image',
      children: ImageView(),
    },
    {
      key: '3',
      label: 'Utilities',
      children: UtilsView(messageApi, actions),
    },
  ];

  const headerStyle: CSSProperties = {
    textAlign: 'left',
    color: '#fff',
    height: 64,
    paddingInline: 48,
    lineHeight: '64px',
    backgroundColor: '#87b7f3',
  };
  const contentStyle: CSSProperties = {
    minHeight: 120,
    lineHeight: '120px',
    // color: 'blue',
    // backgroundColor: '#0958d9',
    paddingInline: 48,
  };
  const footerStyle: CSSProperties = {
    textAlign: 'center',
    color: '#fff',
  };
  const layoutStyle: CSSProperties = {
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(100% - 8px)',
    maxWidth: 'calc(100% - 8px)',
  };

  return (
    <Flex gap="middle" vertical justify="center">
      {contextHolder}
      <Layout style={layoutStyle}>
        <Header style={headerStyle}>
          <div>
            <Row>
              <Col span={2}><a href="https://github.com/windoze/clip-sync" target="_blank"><Image src={"/favicon.ico"} width={64} height={64} className={"inline-block w-12 h-12 mr-2"} alt={'ClipSync'} /></a></Col>
              <Col span={20}><h1 style={headerStyle} className="text-4xl font-bold text-left">Clip Sync</h1></Col>
            </Row>
          </div>
        </Header>
        <Content style={contentStyle}>
          <Tabs defaultActiveKey="1" items={items} />
        </Content>
        <Footer style={footerStyle}>
          <a href='https://github.com/windoze/clip-sync' ><GithubOutlined /></a>
        </Footer>
      </Layout>
    </Flex>
  )
}
