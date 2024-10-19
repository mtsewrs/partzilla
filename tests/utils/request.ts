export function createRequest(url: string | URL) {
  const formData = new FormData();
  const content = '<q id="a"><span id="b">hey!</span></q>';
  const contentType = "text/html;charset=utf-8";
  const blob = new Blob([content], { type: contentType });
  formData.append("file", blob);
  const request = new Request(url, {
    body: formData,
    method: "POST",
  });

  return { request, content, contentType };
}
