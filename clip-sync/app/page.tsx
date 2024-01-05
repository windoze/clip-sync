'use client'


import '../public/antd.min.css';
import '../app/globals.css';
import Image from 'next/image'
import { Flex, Layout, Tabs } from 'antd';
import { SearchableTextHistory } from './ui/text-view';
import { Content, Footer, Header } from 'antd/es/layout/layout';

export default function Home() {
  const items = [
    {
      key: '1',
      label: 'Text',
      children: SearchableTextHistory(),
    },
    {
      key: '2',
      label: 'Image',
      children: 'Image Gallery',
    },
  ];

  const headerStyle = {
    textAlign: 'left',
    color: '#fff',
    height: 64,
    paddingInline: 48,
    lineHeight: '64px',
    backgroundColor: '#4096ff',
  };
  const contentStyle = {
    minHeight: 120,
    lineHeight: '120px',
    // color: '#fff',
    // backgroundColor: '#0958d9',
    paddingInline: 48,
  };
  const footerStyle = {
    textAlign: 'center',
    color: '#fff',
    // backgroundColor: '#4096ff',
  };
  const layoutStyle = {
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(100% - 8px)',
    maxWidth: 'calc(100% - 8px)',
  };

  return (
    <Flex gap="middle" vertical justify="center">
      <Layout style={layoutStyle}>
        <Header style={headerStyle}>
          <div>
            <h1 className="text-5xl font-bold text-left">
              ClipSync
            </h1>
            <p className="mt-4 text-center">
              A clipboard syncing tool
            </p>
          </div>
        </Header>
        <Content style={contentStyle}>
          <Tabs defaultActiveKey="1" items={items} />

        </Content>
        <Footer style={footerStyle}>Footer</Footer>
      </Layout>
    </Flex>
  )
}
