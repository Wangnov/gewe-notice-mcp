# 发送语音消息

## OpenAPI Specification

```yaml
openapi: 3.0.1
info:
  title: ''
  description: ''
  version: 1.0.0
paths:
  /gewe/v2/api/message/postVoice:
    post:
      summary: 发送语音消息
      deprecated: false
      description: ''
      tags:
        - 基础API/消息模块
      parameters:
        - name: X-GEWE-TOKEN
          in: header
          description: ''
          required: true
          example: '{{gewe-token}}'
          schema:
            type: string
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                appId:
                  type: string
                  description: 设备ID
                  additionalProperties: false
                toWxid:
                  type: string
                  description: 好友/群的ID
                voiceUrl:
                  type: string
                  description: 语音文件的链接，仅支持silk格式
                voiceDuration:
                  type: integer
                  description: 语音时长，单位毫秒
              x-apifox-orders:
                - appId
                - toWxid
                - voiceUrl
                - voiceDuration
              required:
                - appId
                - toWxid
                - voiceUrl
                - voiceDuration
            example:
              appId: '{{appid}}'
              toWxid: 34757816141@chatroom
              voiceUrl: >-
                https://scrm-1308498490.cos.ap-shanghai.myqcloud.com/1/silkFile.silk?q-sign-algorithm=sha1&q-ak=AKIDmOkqfDUUDfqjMincBSSAbleGaeQv96mB&q-sign-time=1724227312;2588140912&q-key-time=1724227312;2588140912&q-header-list=&q-url-param-list=&q-signature=7c603355032a67280328c9b898b9e04bdd56e79b
              voiceDuration: 2000
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: object
                properties:
                  ret:
                    type: integer
                  msg:
                    type: string
                  data:
                    type: object
                    properties:
                      toWxid:
                        type: string
                        description: 接收人的wxid
                      createTime:
                        type: integer
                        description: 发送时间
                      msgId:
                        type: integer
                        description: 消息ID
                      newMsgId:
                        type: integer
                        description: 消息ID
                      type:
                        type: integer
                        description: 消息类型
                    required:
                      - toWxid
                      - createTime
                      - msgId
                      - newMsgId
                      - type
                    x-apifox-orders:
                      - toWxid
                      - createTime
                      - msgId
                      - newMsgId
                      - type
                required:
                  - ret
                  - msg
                  - data
                x-apifox-orders:
                  - ret
                  - msg
                  - data
              example:
                ret: 200
                msg: 操作成功
                data:
                  toWxid: 34757816141@chatroom
                  createTime: 1704357563
                  msgId: 640355967
                  newMsgId: 2321462558768366600
                  type: null
          headers: {}
          x-apifox-name: 成功
      security: []
      x-apifox-folder: 基础API/消息模块
      x-apifox-status: released
      x-run-in-apifox: https://app.apifox.com/web/project/3475559/apis/api-139908316-run
components:
  schemas: {}
  securitySchemes: {}
servers:
  - url: http://api.geweapi.com
    description: 测试环境
security: []

```