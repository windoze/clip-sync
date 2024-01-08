import { Button, Divider, Timeline } from "antd";
import { webSocketComponent } from "../lib/api";
import { MessageInstance } from "antd/es/message/interface";
import TextArea from "antd/es/input/TextArea";
import { useTranslation } from "react-i18next";

export function UtilsView(messageApi: MessageInstance, actions: any[]) {
    const { t } = useTranslation();

    function getClipboardText() {
        return ((document!!.getElementById('copy-text')!!) as HTMLInputElement).value;
    }

    function onSendClick() {
        let text = getClipboardText();
        let message = {
            source: '$utilities',
            text: text,
        };
        webSocketComponent.send(JSON.stringify(message));
        messageApi.open({
            type: 'success',
            content: t('copiedMessage'),
            duration: 3,
        });
    }
    return (
        <div>
            <p>{t('copyTextToClipboard')}</p>
            <div>
                <TextArea rows={4} id="copy-text" placeholder={t('inputText')} allowClear autoFocus />
                <Button id="send-button" onClick={onSendClick} style={{ color: 'black' }} >{t('sendText')}</Button>
            </div>
            <Divider type='horizontal' ></Divider>
            <Timeline items={actions} />
        </div>
    );
}
