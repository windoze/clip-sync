'use client'

import Image from 'next/image'
import { Tab, Tabs, TabList, TabPanel } from 'react-tabs';
import 'react-tabs/style/react-tabs.css';
import { SearchableTextHistory } from './ui/text-view';

export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24">
      <div className="z-10 max-w-5xl w-full items-center justify-between font-mono text-sm lg:flex">
        <div className="flex flex-col items-center justify-center">
          <div className="flex flex-col items-center justify-center">
            <h1 className="text-5xl font-bold text-center">
              ClipSync
            </h1>
            <p className="mt-4 text-center">
              A clipboard syncing tool
            </p>
          </div>
          <div>
            <Tabs defaultIndex={0} onSelect={(index) => console.log(index)}>
              <TabList>
                <Tab>Text</Tab>
                <Tab>Image</Tab>
              </TabList>
              <TabPanel>
                {SearchableTextHistory()}
              </TabPanel>
              <TabPanel>
                <p>Image</p>
              </TabPanel>
            </Tabs>
          </div>
        </div>
      </div>
    </main>
  )
}
