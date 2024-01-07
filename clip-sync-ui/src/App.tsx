import "react-image-gallery/styles/css/image-gallery.css";
import './App.css'

import { Flex, Layout, Tabs, message } from 'antd';
import { EntryView, SearchableTextHistory } from './ui/text-view';
import { GithubOutlined } from '@ant-design/icons';
import { CSSProperties, useEffect, useState } from 'react';
import { ImageView } from './ui/image-view';
import { UtilsView } from './ui/utils-view';
import { webSocketComponent } from './lib/api';

const { Header, Footer, Content } = Layout;

function App() {
  const [messageApi, contextHolder] = message.useMessage();
  let [actions, setActions] = useState<any[]>([]);

  function messageBubbleHandler(msg: any) {
    console.log("Page:", msg);
    messageApi.open({
      type: 'info',
      content: `${msg.source} copied '${msg.text}'`,
      duration: 3,
    });
    setActions([
      {
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
    height: 80,
    paddingInline: 48,
    lineHeight: '64px',
    backgroundColor: '#87b7f3',
    display: 'flex',
    alignItems: 'center'
  };
  const contentStyle: CSSProperties = {
    minWidth: 'calc(100vh - 8px)',
    minHeight: 'calc(100vh - 8px)',
    lineHeight: '120px',
    paddingInline: 48,
    alignItems: 'left'
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
    minWidth: 'calc(100vh - 8px)',
    minHeight: 'calc(100vh - 8px)',
    verticalAlign: 'top',
  };

  return (
    <Flex gap="middle" vertical justify="center" style={{ minHeight: '100vh', verticalAlign: 'top' }}>
      {contextHolder}
      < Layout style={layoutStyle}>
        <Header style={headerStyle}>
          <a href="https://github.com/windoze/clip-sync" target="_blank"><img src={"/logo.png"} width={48} height={48} alt={'ClipSync'} /></a>
          <h1 style={headerStyle}>Clip Sync</h1>
        </Header>
        <Content style={contentStyle}>
          <Tabs defaultActiveKey="1" items={items} />
        </Content>
        <Footer style={footerStyle}>
          <a href='https://github.com/windoze/clip-sync' ><GithubOutlined /></a>
        </Footer>
      </Layout >
    </Flex >
  )
}

export default App
