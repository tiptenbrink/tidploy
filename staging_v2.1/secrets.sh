#!/bin/sh
echo $BWS_ACCESS_TOKEN
export POSTGRES_PASSWORD=$(bws secret get 'fad6f227-e0fb-46f9-9380-b0ce0130e122' | jql -r '"value"')
export KEY_PASSWORD=$(bws secret get '02134621-a2cc-4a96-a695-b0ce0131081f' | jql -r '"value"')
export REDIS_PASSWORD=$(bws secret get '5fc28b91-b3c4-4b68-b7aa-b0ce0130fb4f' | jql -r '"value"')
export COMCOM_MAIL_PASS=$(bws secret get '16da2f09-be29-41d3-b47f-b0ce0130ed0f' | jql -r '"value"')
echo $REDIS_PASSWORD