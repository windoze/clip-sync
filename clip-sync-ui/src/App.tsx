import "react-image-gallery/styles/css/image-gallery.css";
import './App.css'

import { Col, Flex, Layout, Row, Tabs, message } from 'antd';
import { EntryView, SearchableTextHistory } from './ui/text-view';
import { GithubOutlined } from '@ant-design/icons';
import { CSSProperties, useEffect, useState } from 'react';
import { ImageView } from './ui/image-view';
import { UtilsView } from './ui/utils-view';
import { webSocketComponent } from './lib/api';
import { useTranslation } from "react-i18next";
import './i18n.ts';

const { Header, Footer, Content } = Layout;

function App() {
  const { t } = useTranslation();

  const [messageApi, contextHolder] = message.useMessage();
  let [actions, setActions] = useState<any[]>([]);

  function messageBubbleHandler(msg: any) {
    console.log("Page:", msg);
    if (msg.text) {
      messageApi.open({
        type: 'info',
        content: t("copyTextMessage", { source: msg.source, text: msg.text }),
        duration: 3,
      });
    } else if (msg.imageurl) {
      messageApi.open({
        type: 'info',
        content: t("copyImageMessage", { source: msg.source }),
        duration: 3,
      });
    }
    setActions([
      {
        children: EntryView(msg, messageApi, false, t),
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
      label: t('labelText'),
      children: SearchableTextHistory(messageApi),
    },
    {
      key: '2',
      label: t('labelImage'),
      children: ImageView(),
    },
    {
      key: '3',
      label: t('labelUtils'),
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
              <Col span={2}><a href="https://github.com/windoze/clip-sync" target="_blank"><img src={"/favicon.ico"} width={64} height={64} className={"inline-block w-12 h-12 mr-2"} alt={'ClipSync'} style={{objectFit: 'contain'}} /></a></Col>
              <Col span={20}><h1 style={headerStyle} className="text-4xl font-bold text-left">Clip&nbsp;Sync</h1></Col>
            </Row>
          </div>
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
