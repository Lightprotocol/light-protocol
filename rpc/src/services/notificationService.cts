// TODO: hook up to better notification service
const accountSid = process.env.TWILIO_ACCOUNT_SID;
const authToken = process.env.TWILIO_AUTH_TOKEN;
const client = require("twilio")(accountSid, authToken);
async function sendTwilio(message: any) {
  try {
    console.log("sending notification to TO1");
    client.messages
      .create({
        from: `whatsapp:${process.env.FROM}`,
        body: message,
        to: `whatsapp:${process.env.TO1}`,
      })
      .then((message: any) => console.log(message.sid));
  } catch (e) {
    console.log(e);
  }

  try {
    console.log("sending notification to TO2");
    client.messages
      .create({
        from: `whatsapp:${process.env.FROM}`,
        body: message,
        to: `whatsapp:${process.env.TO2}`,
      })
      .then((message: any) => console.log(message.sid));
  } catch (e) {
    console.log(e);
  }
  try {
    console.log("sending notification to TO3");
    client.messages
      .create({
        from: `whatsapp:${process.env.FROM}`,
        body: message,
        to: `whatsapp:${process.env.TO3}`,
      })
      .then((message: any) => console.log(message.sid));
  } catch (e) {
    console.log(e);
  }
}

module.exports = {
  sendTwilio,
};
