import { Button, Divider, Timeline } from "antd";
import { webSocketComponent } from "../lib/api";
import { MessageInstance } from "antd/es/message/interface";
import TextArea from "antd/es/input/TextArea";

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
        <div style={{ minHeight: '100vh' }}>
            <p>Send text to clipboards</p>
            <div>
                <TextArea rows={4} id="copy-text" placeholder="input text" allowClear autoFocus />
                <Button type='primary' onClick={onSendClick} style={{ color: 'blue' }}>Send</Button>
            </div>
            <Divider type='horizontal' ></Divider>
            <Timeline items={actions} />
        </div>
    );
}
