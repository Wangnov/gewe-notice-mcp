# 发送文字消息

## OpenAPI Specification

```yaml
openapi: 3.0.1
info:
  title: ''
  description: ''
  version: 1.0.0
paths:
  /gewe/v2/api/message/postText:
    post:
      summary: 发送文字消息
      deprecated: false
      description: |-
        #### 注意
        在群内发送消息@某人时，content中需包含@xxx
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
                content:
                  type: string
                  description: 消息内容
                ats:
                  type: string
                  description: '@的好友，多个英文逗号分隔。群主或管理员@全部的人，则填写''notify@all'''
              x-apifox-orders:
                - appId
                - toWxid
                - content
                - ats
              required:
                - appId
                - toWxid
                - content
            example:
              appId: '{{appid}}'
              toWxid: 34757816141@chatroom
              ats: wxid_phyyedw9xap22
              content: '@猿猴 我在测试艾特内容'
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
                  createTime: 1703841160
                  msgId: 0
                  newMsgId: 3768973957878705000
                  type: 1
          headers: {}
          x-apifox-name: 成功
      security: []
      x-apifox-folder: 基础API/消息模块
      x-apifox-status: released
      x-run-in-apifox: https://app.apifox.com/web/project/3475559/apis/api-139908313-run
components:
  schemas: {}
  securitySchemes: {}
servers:
  - url: http://api.geweapi.com
    description: 测试环境
security: []

```