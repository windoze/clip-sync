import { SearchOutlined, SettingFilled } from "@ant-design/icons";
import { Button, Divider, Input, Space, Timeline } from "antd";
import { WebSocketComponent, webSocketComponent } from "../lib/api";
import { MessageInstance } from "antd/es/message/interface";
import { use, useEffect, useState } from "react";
import TextArea from "antd/es/input/TextArea";

const buttonStyle = {
    color: 'black',
};

export function UtilsView(messageApi: MessageInstance, actions: any[]) {
    function onSendClick() {
        let text = ((document!!.getElementById('copy-text')!!) as HTMLInputElement).value;
        let message = {
            source: '$utilities',
            text: text,
        };
        webSocketComponent.send(JSON.stringify(message));
        messageApi.open({
            type: 'success',
            content: 'Sent text to clipboard',
            duration: 3,
        });
    }
    return (
        <div>
            <p>Send text to clipboards</p>
            <div className="flex flex-row justify-between">
                <TextArea rows={4} id="copy-text" placeholder="input text" allowClear autoFocus />
                <Button type='primary' onClick={onSendClick} style={buttonStyle} >Send</Button>
            </div>
            <Divider type='horizontal' ></Divider>
            <Timeline items={actions} />
        </div>
    );
}
