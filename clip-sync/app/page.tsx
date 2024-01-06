'use client'


import '../public/antd.min.css';
import '../app/globals.css';
import Image from 'next/image'
import { Col, Flex, Layout, Row, Tabs } from 'antd';
import { SearchableTextHistory } from './ui/text-view';
import { Content, Footer, Header } from 'antd/es/layout/layout';
import { GithubOutlined } from '@ant-design/icons';

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
    backgroundColor: '#87b7f3',
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
            <Row>
              <Col span={2}><img src="./favicon.ico" className="inline-block w-12 h-12 mr-2" /></Col>
              <Col span={20}><h1 style={headerStyle} className="text-4xl font-bold text-left">Clip Sync</h1></Col>
            </Row>
            <p className="mt-4 text-center">
              A clipboard syncing tool
            </p>
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
